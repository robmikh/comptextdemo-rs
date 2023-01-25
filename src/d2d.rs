use windows::core::{Interface, Result};
use windows::Win32::Graphics::{
    Direct2D::{
        D2D1CreateFactory, ID2D1Device, ID2D1Factory1, D2D1_FACTORY_OPTIONS,
        D2D1_FACTORY_TYPE_SINGLE_THREADED,
    },
    Direct3D11::ID3D11Device,
    Dxgi::IDXGIDevice,
};

pub fn create_d2d_factory(options: D2D1_FACTORY_OPTIONS) -> Result<ID2D1Factory1> {
    let result = unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, Some(&options))? };
    Ok(result)
}

pub fn create_d2d_device(factory: &ID2D1Factory1, device: &ID3D11Device) -> Result<ID2D1Device> {
    let dxgi_device: IDXGIDevice = device.cast()?;
    unsafe { factory.CreateDevice(&dxgi_device) }
}
