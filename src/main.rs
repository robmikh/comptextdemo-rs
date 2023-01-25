mod composition;
mod d2d;
mod d3d;
mod handle;
mod interop;
mod numerics;
mod window;

use std::time::Duration;

use composition::{draw_into_surface, CompositorInterop};
use d2d::{create_d2d_device, create_d2d_factory};
use d3d::create_d3d_device;
use interop::{
    create_dispatcher_queue_controller_for_current_thread,
    shutdown_dispatcher_queue_controller_and_wait,
};
use window::Window;
use windows::{
    core::{Result, HRESULT, HSTRING},
    w,
    Foundation::Numerics::{Matrix3x2, Vector2, Vector3},
    Graphics::{
        DirectX::{DirectXAlphaMode, DirectXPixelFormat},
        SizeInt32,
    },
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::{
            Direct2D::{
                Common::{D2D1_COLOR_F, D2D_POINT_2F},
                ID2D1DeviceContext, D2D1_DEBUG_LEVEL_INFORMATION, D2D1_DRAW_TEXT_OPTIONS_NONE,
                D2D1_FACTORY_OPTIONS,
            },
            Direct3D11::{D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_CREATE_DEVICE_DEBUG},
            DirectWrite::{
                DWriteCreateFactory, IDWriteFactory, IDWriteFontCollection,
                DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_WEIGHT_NORMAL,
            },
        },
        System::WinRT::{RoInitialize, RO_INIT_SINGLETHREADED},
        UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, TranslateMessage, MSG},
    },
    UI::{
        Color,
        Composition::{AnimationIterationBehavior, Compositor},
    },
};

fn run() -> Result<()> {
    unsafe { RoInitialize(RO_INIT_SINGLETHREADED)? };
    let controller = create_dispatcher_queue_controller_for_current_thread()?;

    let window_width = 800;
    let window_height = 600;

    let compositor = Compositor::new()?;
    let root = compositor.CreateSpriteVisual()?;
    root.SetBrush(&compositor.CreateColorBrushWithColor(Color {
        A: 255,
        R: 255,
        G: 255,
        B: 255,
    })?)?;
    root.SetRelativeSizeAdjustment(Vector2::new(1.0, 1.0))?;

    let window = Window::new("Composition Text Demo", window_width, window_height)?;
    let target = window.create_window_target(&compositor, false)?;
    target.SetRoot(&root)?;

    // Init D3D and D2D
    let mut flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;
    if cfg!(feature = "debug") {
        flags |= D3D11_CREATE_DEVICE_DEBUG;
    }
    let d3d_device = create_d3d_device(flags)?;
    let options = {
        let mut options = D2D1_FACTORY_OPTIONS::default();
        if cfg!(feature = "debug") {
            options.debugLevel = D2D1_DEBUG_LEVEL_INFORMATION;
        }
        options
    };
    let d2d_factory = create_d2d_factory(options)?;
    let d2d_device = create_d2d_device(&d2d_factory, &d3d_device)?;
    let comp_graphics = compositor.create_graphics_device_from_d2d_device(&d2d_device)?;

    // Init DWrite
    let dwrite_factory: IDWriteFactory =
        unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? };
    let font_collection: IDWriteFontCollection = unsafe {
        let mut font_collection = None;
        dwrite_factory.GetSystemFontCollection(&mut font_collection, false)?;
        font_collection.unwrap()
    };
    let text_format = unsafe {
        let font_name = w!("Comic Sans MS");
        let locale = w!("en-us");
        dwrite_factory.CreateTextFormat(
            font_name,
            &font_collection,
            DWRITE_FONT_WEIGHT_NORMAL,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            36.0,
            locale,
        )?
    };

    // Create our text layout
    let text = "Hello, World!";
    let text_layout = unsafe {
        let text_data: Vec<_> = text.encode_utf16().collect();
        dwrite_factory.CreateTextLayout(&text_data, &text_format, 400.0, 0.0)?
    };
    let metrics = unsafe { text_layout.GetOverhangMetrics()? };
    let max_width = unsafe { text_layout.GetMaxWidth() };
    let max_height = unsafe { text_layout.GetMaxHeight() };

    let text_rect = RECT {
        left: 0,
        top: 0,
        right: (metrics.right + max_width + -metrics.left) as i32,
        bottom: (metrics.bottom + max_height + -metrics.top) as i32,
    };
    let text_size = SizeInt32 {
        Width: text_rect.right - text_rect.left,
        Height: text_rect.bottom - text_rect.top,
    };

    // Create a visual for our text
    let visual = compositor.CreateSpriteVisual()?;
    visual.SetAnchorPoint(Vector2::new(0.5, 0.5))?;
    visual.SetRelativeOffsetAdjustment(Vector3::new(0.5, 0.5, 0.0))?;
    visual.SetSize(Vector2::new(
        text_size.Width as f32,
        text_size.Height as f32,
    ))?;
    root.Children()?.InsertAtTop(&visual)?;

    // Setup our basic brush
    let surface = comp_graphics.CreateDrawingSurface2(
        text_size,
        DirectXPixelFormat::A8UIntNormalized,
        DirectXAlphaMode::Premultiplied,
    )?;
    let basic_brush = compositor.CreateSurfaceBrushWithSurface(&surface)?;
    draw_into_surface(
        &surface,
        |d2d_context: &ID2D1DeviceContext, offset| unsafe {
            d2d_context.SetTransform(&Matrix3x2::translation(offset.x as f32, offset.y as f32));

            let d2d_brush = {
                let color = D2D1_COLOR_F {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                };
                d2d_context
                    .CreateSolidColorBrush(&color, None)
                    .expect("Failed to create color brush!")
            };

            d2d_context.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
            d2d_context.DrawTextLayout(
                D2D_POINT_2F { x: 0.0, y: 0.0 },
                &text_layout,
                &d2d_brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
            );
        },
    )?;

    // Create our mask brush
    let mask_brush = compositor.CreateMaskBrush()?;
    let text_color_brush = compositor.CreateColorBrushWithColor(Color {
        A: 255,
        R: 255,
        G: 0,
        B: 0,
    })?;
    mask_brush.SetSource(&text_color_brush)?;
    mask_brush.SetMask(&basic_brush)?;
    visual.SetBrush(&mask_brush)?;

    // Animate our text color
    let animation = compositor.CreateColorKeyFrameAnimation()?;
    animation.InsertKeyFrame(
        0.0,
        Color {
            A: 255,
            R: 255,
            G: 0,
            B: 0,
        },
    )?;
    animation.InsertKeyFrame(
        0.25,
        Color {
            A: 255,
            R: 0,
            G: 255,
            B: 0,
        },
    )?;
    animation.InsertKeyFrame(
        0.5,
        Color {
            A: 255,
            R: 0,
            G: 0,
            B: 255,
        },
    )?;
    animation.InsertKeyFrame(
        0.75,
        Color {
            A: 255,
            R: 255,
            G: 255,
            B: 0,
        },
    )?;
    animation.InsertKeyFrame(
        1.0,
        Color {
            A: 255,
            R: 255,
            G: 0,
            B: 0,
        },
    )?;
    animation.SetDuration(Duration::from_secs(3).into())?;
    animation.SetIterationBehavior(AnimationIterationBehavior::Forever)?;
    text_color_brush.StartAnimation(&HSTRING::from("Color"), &animation)?;

    // Add a border for debugging
    let border = compositor.CreateSpriteVisual()?;
    let border_size = 2.0;
    border.SetSize(Vector2::new(border_size * 2.0, border_size * 2.0))?;
    border.SetRelativeSizeAdjustment(Vector2::one())?;
    border.SetOffset(Vector3::new(-border_size, -border_size, 0.0))?;
    let border_brush = compositor.CreateNineGridBrush()?;
    border.SetBrush(&border_brush)?;
    border_brush.SetInsets(border_size)?;
    border_brush.SetIsCenterHollow(true)?;
    border_brush.SetSource(&compositor.CreateColorBrushWithColor(Color {
        A: 255,
        R: 255,
        G: 0,
        B: 0,
    })?)?;
    visual.Children()?.InsertAtTop(&border)?;

    // Pump messages and exit
    let mut message = MSG::default();
    unsafe {
        while GetMessageW(&mut message, HWND(0), 0, 0).into() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    let code = shutdown_dispatcher_queue_controller_and_wait(&controller, message.wParam.0 as i32)?;
    if code != 0 {
        Err(HRESULT(message.wParam.0 as i32).into())
    } else {
        Ok(())
    }
}

fn main() {
    let result = run();

    // We do this for nicer HRESULT printing when errors occur.
    if let Err(error) = result {
        error.code().unwrap();
    }
}
