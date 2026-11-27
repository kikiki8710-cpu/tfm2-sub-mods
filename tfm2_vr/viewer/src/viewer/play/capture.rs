//! 게임 창 화면 받기(Windows.Graphics.Capture). 창이 다른 창에 가려도 된다(최소화는 안 됨).
use anyhow::{Context, Result};
use windows::{core::{factory, Interface}, Foundation::TypedEventHandler, Graphics::{Capture::*, DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat}, SizeInt32}, Win32::{Foundation::HWND, Graphics::{Direct3D::D3D_SRV_DIMENSION_TEXTURE2D, Direct3D11::*, Dxgi::{Common::*, IDXGIDevice}}, System::WinRT::{Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess}, Graphics::Capture::IGraphicsCaptureItemInterop, RoInitialize, RO_INIT_MULTITHREADED}}};
use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

pub struct Capture { device: IDirect3DDevice, pool: Direct3D11CaptureFramePool, session: GraphicsCaptureSession, _item: GraphicsCaptureItem, size: SizeInt32, texture: Option<ID3D11Texture2D>, pub view: Option<ID3D11ShaderResourceView>, pub width: u32, pub height: u32, closed: Arc<AtomicBool>, pub frames: u64 }
impl Capture {
    pub fn new(d3d: &ID3D11Device, hwnd: HWND) -> Result<Self> {
        unsafe {
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
            let dxgi: IDXGIDevice = d3d.cast()?;
            let device: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgi)?.cast()?;
            let interop: IGraphicsCaptureItemInterop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = interop.CreateForWindow(hwnd)?;
            let size = item.Size()?;
            let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&device, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size)?;
            let session = pool.CreateCaptureSession(&item)?;
            let _ = session.SetIsCursorCaptureEnabled(false);
            let _ = session.SetIsBorderRequired(false);
            let closed = Arc::new(AtomicBool::new(false));
            let flag = closed.clone();
            item.Closed(&TypedEventHandler::new(move |_, _| { flag.store(true, Ordering::SeqCst); Ok(()) }))?;
            session.StartCapture()?;
            Ok(Self { device, pool, session, _item: item, size, texture: None, view: None, width: 0, height: 0, closed, frames: 0 })
        }
    }
    pub fn closed(&self) -> bool { self.closed.load(Ordering::SeqCst) }
    /// 새 화면이 있으면 내 텍스처(밉맵 포함)로 복사한다.
    pub fn poll(&mut self, d3d: &ID3D11Device, context: &ID3D11DeviceContext) -> Result<bool> {
        let mut newest = None;
        while let Ok(f) = self.pool.TryGetNextFrame() { newest = Some(f); }
        let Some(frame) = newest else { return Ok(false) };
        let size = frame.ContentSize()?;
        let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
        unsafe {
            let source: ID3D11Texture2D = access.GetInterface()?;
            let (w, h) = (size.Width.max(1) as u32, size.Height.max(1) as u32);
            if self.texture.is_none() || (w, h) != (self.width, self.height) {
                let desc = D3D11_TEXTURE2D_DESC { Width: w, Height: h, MipLevels: 0, ArraySize: 1, Format: DXGI_FORMAT_B8G8R8A8_TYPELESS, SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 }, Usage: D3D11_USAGE_DEFAULT, BindFlags: (D3D11_BIND_SHADER_RESOURCE | D3D11_BIND_RENDER_TARGET).0 as u32, MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32, ..Default::default() };
                let mut t = None;
                d3d.CreateTexture2D(&desc, None, Some(&mut t))?;
                let t = t.context("패널 텍스처")?;
                let mut v = None;
                d3d.CreateShaderResourceView(&t, Some(&D3D11_SHADER_RESOURCE_VIEW_DESC { Format: DXGI_FORMAT_B8G8R8A8_UNORM_SRGB, ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D, Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 { Texture2D: D3D11_TEX2D_SRV { MostDetailedMip: 0, MipLevels: u32::MAX } } }), Some(&mut v))?;
                (self.texture, self.view, self.width, self.height) = (Some(t), v, w, h);
            }
            let mut sd = D3D11_TEXTURE2D_DESC::default();
            source.GetDesc(&mut sd);
            let b = D3D11_BOX { left: 0, top: 0, front: 0, right: w.min(sd.Width), bottom: h.min(sd.Height), back: 1 };
            context.CopySubresourceRegion(self.texture.as_ref().unwrap(), 0, 0, 0, 0, &source, 0, Some(&b));
            context.GenerateMips(self.view.as_ref().unwrap());
        }
        if size != self.size { self.size = size; self.pool.Recreate(&self.device, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size)?; }
        self.frames += 1;
        Ok(true)
    }
}
impl Drop for Capture { fn drop(&mut self) { let _ = self.session.Close(); let _ = self.pool.Close(); } }
