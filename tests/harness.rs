use std::{cmp::Ordering, time::Instant};

use imgui_wgpu::{Renderer, RendererConfig};

use imgui::{Context, Io, Key, Ui};
use wgpu::{Backends, Device, Instance, InstanceDescriptor, Surface, SurfaceConfiguration};
use winit::{
    dpi::LogicalSize,
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, TouchPhase,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub trait Test {
    fn render(&mut self, ui: &Ui) -> bool;
}

pub fn test<T: Test + 'static>(mut test_cases: Vec<T>) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("practice-tool-core test harness")
        .with_inner_size(LogicalSize::new(1280, 800))
        .build(&event_loop)
        .expect("WindowBuilder::build");

    let instance =
        Instance::new(InstanceDescriptor { backends: Backends::PRIMARY, ..Default::default() });

    let surface = unsafe { instance.create_surface(&window).expect("Instance::create_surface") };
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .expect("Instance::request_adapter");

    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
            .expect("Adapter::request_device");

    let surface_desc = SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: 1280,
        height: 800,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
    };

    surface.configure(&device, &surface_desc);

    let mut ctx = Context::create();
    ctx.set_ini_filename(None);

    let renderer_config =
        RendererConfig { texture_format: surface_desc.format, ..Default::default() };
    let mut renderer = Renderer::new(&mut ctx, &device, &queue, renderer_config);

    let mut last_frame = Instant::now();
    let clear_color = wgpu::Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => {
                handle_window_event(event, control_flow, ctx.io_mut(), &surface, &device)
            },
            Event::MainEventsCleared => window.request_redraw(),
            Event::RedrawEventsCleared => {
                let now = Instant::now();
                ctx.io_mut().update_delta_time(now - last_frame);
                last_frame = now;

                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("dropped frame: {e:?}");
                        return;
                    },
                };

                let ui = ctx.frame();

                for test_case in &mut test_cases {
                    test_case.render(ui);
                }

                let mut encoder: wgpu::CommandEncoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(clear_color),
                            store: true,
                        },
                    })],
                    depth_stencil_attachment: None,
                });

                renderer
                    .render(ctx.render(), &queue, &device, &mut rpass)
                    .expect("Rendering failed");

                drop(rpass);

                queue.submit(Some(encoder.finish()));

                frame.present();
            },
            _ => (),
        }
    });
}

fn handle_window_event(
    event: WindowEvent,
    control_flow: &mut ControlFlow,
    io: &mut Io,
    surface: &Surface,
    device: &Device,
) {
    match event {
        WindowEvent::Resized(size) => {
            let surface_desc = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
            };

            surface.configure(device, &surface_desc);
            io.display_size = [size.width as f32, size.height as f32];
        },
        WindowEvent::KeyboardInput {
            input:
                KeyboardInput {
                    virtual_keycode: Some(VirtualKeyCode::Escape),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        }
        | WindowEvent::CloseRequested => {
            *control_flow = ControlFlow::Exit;
        },
        WindowEvent::KeyboardInput {
            input: KeyboardInput { virtual_keycode: Some(vk), state, .. },
            ..
        } => {
            let pressed = state == ElementState::Pressed;
            handle_key_modifier(io, vk, pressed);
            if let Some(key) = to_imgui_key(vk) {
                io.add_key_event(key, pressed);
            }
        },
        WindowEvent::ModifiersChanged(modifiers) => {
            io.add_key_event(Key::ModShift, modifiers.shift());
            io.add_key_event(Key::ModCtrl, modifiers.ctrl());
            io.add_key_event(Key::ModAlt, modifiers.alt());
            io.add_key_event(Key::ModSuper, modifiers.logo());
        },
        WindowEvent::CursorMoved { position, .. } => {
            io.add_mouse_pos_event([position.x as f32, position.y as f32]);
        },
        WindowEvent::MouseWheel { delta, phase: TouchPhase::Moved, .. } => {
            let (h, v) = match delta {
                MouseScrollDelta::LineDelta(h, v) => (h, v),
                MouseScrollDelta::PixelDelta(pos) => {
                    let h = match pos.x.partial_cmp(&0.0) {
                        Some(Ordering::Greater) => 1.0,
                        Some(Ordering::Less) => -1.0,
                        _ => 0.0,
                    };
                    let v = match pos.y.partial_cmp(&0.0) {
                        Some(Ordering::Greater) => 1.0,
                        Some(Ordering::Less) => -1.0,
                        _ => 0.0,
                    };
                    (h, v)
                },
            };
            io.add_mouse_wheel_event([h, v]);
        },
        WindowEvent::MouseInput { state, button, .. } => {
            if let Some(mb) = to_imgui_mouse_button(button) {
                let pressed = state == ElementState::Pressed;
                io.add_mouse_button_event(mb, pressed);
            }
        },
        WindowEvent::Focused(newly_focused) => {
            if !newly_focused {
                io.app_focus_lost = true;
            }
        },
        _ => (),
    }
}

fn to_imgui_mouse_button(button: MouseButton) -> Option<imgui::MouseButton> {
    match button {
        MouseButton::Left | MouseButton::Other(0) => Some(imgui::MouseButton::Left),
        MouseButton::Right | MouseButton::Other(1) => Some(imgui::MouseButton::Right),
        MouseButton::Middle | MouseButton::Other(2) => Some(imgui::MouseButton::Middle),
        MouseButton::Other(3) => Some(imgui::MouseButton::Extra1),
        MouseButton::Other(4) => Some(imgui::MouseButton::Extra2),
        _ => None,
    }
}

fn to_imgui_key(keycode: VirtualKeyCode) -> Option<Key> {
    match keycode {
        VirtualKeyCode::Tab => Some(Key::Tab),
        VirtualKeyCode::Left => Some(Key::LeftArrow),
        VirtualKeyCode::Right => Some(Key::RightArrow),
        VirtualKeyCode::Up => Some(Key::UpArrow),
        VirtualKeyCode::Down => Some(Key::DownArrow),
        VirtualKeyCode::PageUp => Some(Key::PageUp),
        VirtualKeyCode::PageDown => Some(Key::PageDown),
        VirtualKeyCode::Home => Some(Key::Home),
        VirtualKeyCode::End => Some(Key::End),
        VirtualKeyCode::Insert => Some(Key::Insert),
        VirtualKeyCode::Delete => Some(Key::Delete),
        VirtualKeyCode::Back => Some(Key::Backspace),
        VirtualKeyCode::Space => Some(Key::Space),
        VirtualKeyCode::Return => Some(Key::Enter),
        VirtualKeyCode::Escape => Some(Key::Escape),
        VirtualKeyCode::LControl => Some(Key::LeftCtrl),
        VirtualKeyCode::LShift => Some(Key::LeftShift),
        VirtualKeyCode::LAlt => Some(Key::LeftAlt),
        VirtualKeyCode::LWin => Some(Key::LeftSuper),
        VirtualKeyCode::RControl => Some(Key::RightCtrl),
        VirtualKeyCode::RShift => Some(Key::RightShift),
        VirtualKeyCode::RAlt => Some(Key::RightAlt),
        VirtualKeyCode::RWin => Some(Key::RightSuper),
        //VirtualKeyCode::Menu => Some(Key::Menu), // TODO: find out if there is a Menu key in winit
        VirtualKeyCode::Key0 => Some(Key::Alpha0),
        VirtualKeyCode::Key1 => Some(Key::Alpha1),
        VirtualKeyCode::Key2 => Some(Key::Alpha2),
        VirtualKeyCode::Key3 => Some(Key::Alpha3),
        VirtualKeyCode::Key4 => Some(Key::Alpha4),
        VirtualKeyCode::Key5 => Some(Key::Alpha5),
        VirtualKeyCode::Key6 => Some(Key::Alpha6),
        VirtualKeyCode::Key7 => Some(Key::Alpha7),
        VirtualKeyCode::Key8 => Some(Key::Alpha8),
        VirtualKeyCode::Key9 => Some(Key::Alpha9),
        VirtualKeyCode::A => Some(Key::A),
        VirtualKeyCode::B => Some(Key::B),
        VirtualKeyCode::C => Some(Key::C),
        VirtualKeyCode::D => Some(Key::D),
        VirtualKeyCode::E => Some(Key::E),
        VirtualKeyCode::F => Some(Key::F),
        VirtualKeyCode::G => Some(Key::G),
        VirtualKeyCode::H => Some(Key::H),
        VirtualKeyCode::I => Some(Key::I),
        VirtualKeyCode::J => Some(Key::J),
        VirtualKeyCode::K => Some(Key::K),
        VirtualKeyCode::L => Some(Key::L),
        VirtualKeyCode::M => Some(Key::M),
        VirtualKeyCode::N => Some(Key::N),
        VirtualKeyCode::O => Some(Key::O),
        VirtualKeyCode::P => Some(Key::P),
        VirtualKeyCode::Q => Some(Key::Q),
        VirtualKeyCode::R => Some(Key::R),
        VirtualKeyCode::S => Some(Key::S),
        VirtualKeyCode::T => Some(Key::T),
        VirtualKeyCode::U => Some(Key::U),
        VirtualKeyCode::V => Some(Key::V),
        VirtualKeyCode::W => Some(Key::W),
        VirtualKeyCode::X => Some(Key::X),
        VirtualKeyCode::Y => Some(Key::Y),
        VirtualKeyCode::Z => Some(Key::Z),
        VirtualKeyCode::F1 => Some(Key::F1),
        VirtualKeyCode::F2 => Some(Key::F2),
        VirtualKeyCode::F3 => Some(Key::F3),
        VirtualKeyCode::F4 => Some(Key::F4),
        VirtualKeyCode::F5 => Some(Key::F5),
        VirtualKeyCode::F6 => Some(Key::F6),
        VirtualKeyCode::F7 => Some(Key::F7),
        VirtualKeyCode::F8 => Some(Key::F8),
        VirtualKeyCode::F9 => Some(Key::F9),
        VirtualKeyCode::F10 => Some(Key::F10),
        VirtualKeyCode::F11 => Some(Key::F11),
        VirtualKeyCode::F12 => Some(Key::F12),
        VirtualKeyCode::Apostrophe => Some(Key::Apostrophe),
        VirtualKeyCode::Comma => Some(Key::Comma),
        VirtualKeyCode::Minus => Some(Key::Minus),
        VirtualKeyCode::Period => Some(Key::Period),
        VirtualKeyCode::Slash => Some(Key::Slash),
        VirtualKeyCode::Semicolon => Some(Key::Semicolon),
        VirtualKeyCode::Equals => Some(Key::Equal),
        VirtualKeyCode::LBracket => Some(Key::LeftBracket),
        VirtualKeyCode::Backslash => Some(Key::Backslash),
        VirtualKeyCode::RBracket => Some(Key::RightBracket),
        VirtualKeyCode::Grave => Some(Key::GraveAccent),
        VirtualKeyCode::Capital => Some(Key::CapsLock),
        VirtualKeyCode::Scroll => Some(Key::ScrollLock),
        VirtualKeyCode::Numlock => Some(Key::NumLock),
        VirtualKeyCode::Snapshot => Some(Key::PrintScreen),
        VirtualKeyCode::Pause => Some(Key::Pause),
        VirtualKeyCode::Numpad0 => Some(Key::Keypad0),
        VirtualKeyCode::Numpad1 => Some(Key::Keypad1),
        VirtualKeyCode::Numpad2 => Some(Key::Keypad2),
        VirtualKeyCode::Numpad3 => Some(Key::Keypad3),
        VirtualKeyCode::Numpad4 => Some(Key::Keypad4),
        VirtualKeyCode::Numpad5 => Some(Key::Keypad5),
        VirtualKeyCode::Numpad6 => Some(Key::Keypad6),
        VirtualKeyCode::Numpad7 => Some(Key::Keypad7),
        VirtualKeyCode::Numpad8 => Some(Key::Keypad8),
        VirtualKeyCode::Numpad9 => Some(Key::Keypad9),
        VirtualKeyCode::NumpadDecimal => Some(Key::KeypadDecimal),
        VirtualKeyCode::NumpadDivide => Some(Key::KeypadDivide),
        VirtualKeyCode::NumpadMultiply => Some(Key::KeypadMultiply),
        VirtualKeyCode::NumpadSubtract => Some(Key::KeypadSubtract),
        VirtualKeyCode::NumpadAdd => Some(Key::KeypadAdd),
        VirtualKeyCode::NumpadEnter => Some(Key::KeypadEnter),
        VirtualKeyCode::NumpadEquals => Some(Key::KeypadEqual),
        _ => None,
    }
}

fn handle_key_modifier(io: &mut Io, key: VirtualKeyCode, down: bool) {
    if key == VirtualKeyCode::LShift || key == VirtualKeyCode::RShift {
        io.add_key_event(imgui::Key::ModShift, down);
    } else if key == VirtualKeyCode::LControl || key == VirtualKeyCode::RControl {
        io.add_key_event(imgui::Key::ModCtrl, down);
    } else if key == VirtualKeyCode::LAlt || key == VirtualKeyCode::RAlt {
        io.add_key_event(imgui::Key::ModAlt, down);
    } else if key == VirtualKeyCode::LWin || key == VirtualKeyCode::RWin {
        io.add_key_event(imgui::Key::ModSuper, down);
    }
}
