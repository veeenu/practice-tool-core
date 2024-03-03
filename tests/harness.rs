use std::time::Instant;

use glow::HasContext;
use imgui::{Context, Ui};
use imgui_glow_renderer::AutoRenderer;
use imgui_sdl2_support::SdlPlatform;
use sdl2::event::Event;
use sdl2::video::GLProfile;

pub type Test = dyn FnMut(&Ui);

pub fn test(mut test_cases: Vec<Box<Test>>) {
    let sdl = sdl2::init().unwrap();
    let video_subsystem = sdl.video().unwrap();

    let gl_attr = video_subsystem.gl_attr();

    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_profile(GLProfile::Core);

    let window = video_subsystem
        .window("test harness", 1280, 720)
        .allow_highdpi()
        .opengl()
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let gl_context = window.gl_create_context().unwrap();
    window.gl_make_current(&gl_context).unwrap();

    window.subsystem().gl_set_swap_interval(1).unwrap();

    let gl = unsafe {
        glow::Context::from_loader_function(|s| window.subsystem().gl_get_proc_address(s) as _)
    };

    unsafe { gl.clear_color(0.26, 0.35, 0.38, 1.0) };

    let mut ctx = Context::create();
    ctx.set_ini_filename(None);

    let mut platform = SdlPlatform::init(&mut ctx);
    let mut renderer = AutoRenderer::initialize(gl, &mut ctx).unwrap();
    let mut event_pump = sdl.event_pump().unwrap();

    let mut last = Instant::now();
    let mut elapsed = last.elapsed();

    'main: loop {
        for event in event_pump.poll_iter() {
            platform.handle_event(&mut ctx, &event);

            if let Event::Quit { .. } = event {
                break 'main;
            }
        }

        platform.prepare_frame(&mut ctx, &window, &event_pump);

        let ui = ctx.new_frame();

        ui.window("##test_window").build(|| {
            ui.text(format!("FPS: {:.2}", 1. / elapsed.as_secs_f64()));

            elapsed = elapsed.mul_f64(0.8) + last.elapsed().mul_f64(0.2);
            last = Instant::now();

            for test_case in &mut test_cases {
                test_case(ui);
            }
        });

        let draw_data = ctx.render();

        unsafe { renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };
        renderer.render(draw_data).unwrap();

        window.gl_swap_window();
    }
}
