use windows::{
    core::{IUnknown, Interface, Result},
    Win32::{
        Foundation::{POINT, RECT, SIZE},
        Graphics::{Direct2D::ID2D1Device, Direct3D11::ID3D11Device, Dxgi::IDXGISwapChain1},
        System::WinRT::Composition::{ICompositionDrawingSurfaceInterop, ICompositorInterop},
    },
    UI::Composition::{
        CompositionDrawingSurface, CompositionGraphicsDevice, Compositor, ICompositionSurface,
    },
};

pub trait CompositorInterop {
    fn create_graphics_device_from_d3d_device(
        &self,
        device: &ID3D11Device,
    ) -> Result<CompositionGraphicsDevice>;
    fn create_graphics_device_from_d2d_device(
        &self,
        device: &ID2D1Device,
    ) -> Result<CompositionGraphicsDevice>;
    fn create_composition_surface_for_swap_chain(
        &self,
        swap_chain: &IDXGISwapChain1,
    ) -> Result<ICompositionSurface>;
}

impl CompositorInterop for Compositor {
    fn create_graphics_device_from_d3d_device(
        &self,
        device: &ID3D11Device,
    ) -> Result<CompositionGraphicsDevice> {
        let interop: ICompositorInterop = self.cast()?;
        let unknown: IUnknown = device.cast()?;
        unsafe { interop.CreateGraphicsDevice(&unknown) }
    }

    fn create_graphics_device_from_d2d_device(
        &self,
        device: &ID2D1Device,
    ) -> Result<CompositionGraphicsDevice> {
        let interop: ICompositorInterop = self.cast()?;
        let unknown: IUnknown = device.cast()?;
        unsafe { interop.CreateGraphicsDevice(&unknown) }
    }

    fn create_composition_surface_for_swap_chain(
        &self,
        swap_chain: &IDXGISwapChain1,
    ) -> Result<ICompositionSurface> {
        let interop: ICompositorInterop = self.cast()?;
        let unknown: IUnknown = swap_chain.cast()?;
        unsafe { interop.CreateCompositionSurfaceForSwapChain(&unknown) }
    }
}

pub trait CompositionDrawingSurfaceInterop {
    fn resize(&self, size: &SIZE) -> Result<()>;
    fn begin_draw<T: Interface>(&self, update_rect: Option<&RECT>) -> Result<(T, POINT)>;
    fn end_draw(&self) -> Result<()>;
}

impl CompositionDrawingSurfaceInterop for CompositionDrawingSurface {
    fn resize(&self, size: &SIZE) -> Result<()> {
        let interop: ICompositionDrawingSurfaceInterop = self.cast()?;
        unsafe { interop.Resize(*size) }
    }

    fn begin_draw<UpdateObject: Interface>(
        &self,
        update_rect: Option<&RECT>,
    ) -> Result<(UpdateObject, POINT)> {
        let interop: ICompositionDrawingSurfaceInterop = self.cast()?;
        let update_rect = if let Some(update_rect) = update_rect {
            Some(update_rect as *const _)
        } else {
            None
        };
        unsafe {
            let mut update_offset = POINT::default();
            let update_object =
                interop.BeginDraw::<UpdateObject>(update_rect, &mut update_offset)?;
            Ok((update_object, update_offset))
        }
    }

    fn end_draw(&self) -> Result<()> {
        let interop: ICompositionDrawingSurfaceInterop = self.cast()?;
        unsafe { interop.EndDraw() }
    }
}

pub fn draw_into_surface<UpdateObject: Interface, F: FnOnce(&UpdateObject, &POINT)>(
    surface: &CompositionDrawingSurface,
    draw: F,
) -> Result<()> {
    let (update_object, update_offset) = surface.begin_draw(None)?;
    draw(&update_object, &update_offset);
    surface.end_draw()
}
