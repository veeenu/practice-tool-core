use std::f32::consts::PI;

use imgui::sys::{
    igCalcTextSize, igGetForegroundDrawList, ImDrawList_AddText_Vec2, ImDrawList_PathArcTo,
    ImDrawList_PathFillConvex, ImVec2,
};
use imgui::{ImColor32, StyleColor};

/// # Safety
///
/// This method uses functions from imgui_sys that are compatible with the
/// lifetime of holding a &Ui reference.
unsafe fn draw_slice(
    ui: &imgui::Ui,
    txt: &str,
    angle_base: f32,
    (angle_min, angle_max): (f32, f32),
    (radius_min, radius_max): (f32, f32),
    is_active: bool,
) {
    const GAP: f32 = 3.0;

    let [x, y] = ui.io().display_size;
    let center = ImVec2 { x: x * 0.5, y: y * 0.5 };

    let radius_mid = (radius_max + radius_min) * 0.5;
    let gap1 = GAP / radius_max;
    let gap2 = GAP / radius_min;

    let draw_lists = igGetForegroundDrawList();

    let button_color = ImColor32::from(ui.style_color(if is_active {
        StyleColor::ButtonActive
    } else {
        StyleColor::Button
    }));

    ImDrawList_PathArcTo(draw_lists, center, radius_max, angle_min + gap1, angle_max - gap1, 0);
    ImDrawList_PathArcTo(draw_lists, center, radius_min, angle_max - gap2, angle_min + gap2, 0);
    ImDrawList_PathFillConvex(draw_lists, button_color.to_bits());

    let color = ImColor32::from(ui.style_color(StyleColor::Text)).to_bits();
    let color2 = ImColor32::from(ui.style_color(StyleColor::WindowBg)).to_bits();
    let text_start = txt.as_ptr();
    let text_end = text_start.add(txt.len());
    let mut text_size = ImVec2 { x: 0.0, y: 0.0 };
    igCalcTextSize(&mut text_size, text_start as _, text_end as _, false, 0.0);

    let text_pos = ImVec2 {
        x: center.x + radius_mid * angle_base.cos() - text_size.x * 0.5,
        y: center.y + radius_mid * angle_base.sin() - text_size.y * 0.5,
    };

    ImDrawList_AddText_Vec2(
        draw_lists,
        ImVec2 { x: text_pos.x + 1.0, y: text_pos.y + 1.0 },
        color2,
        text_start as _,
        text_end as _,
    );
    ImDrawList_AddText_Vec2(draw_lists, text_pos, color, text_start as _, text_end as _);
}

pub fn radial_menu(
    ui: &imgui::Ui,
    elements: &[&str],
    pos: ImVec2,
    radius_min: f32,
    radius_max: f32,
) -> Option<usize> {
    let mut selected = None;
    let count = elements.len();
    for (index, txt) in elements.iter().enumerate() {
        let slice_angle = PI * 2.0 / (count as f32);
        let angle_base = slice_angle * (index as f32) - PI * 0.5;
        let angle_base = if angle_base < 0.0 { angle_base + 2.0 * PI } else { angle_base };
        let angle_min = angle_base - slice_angle * 0.5;
        let angle_max = angle_base + slice_angle * 0.5;

        let angle_of_pos = f32::atan2(pos.y, pos.x);
        let angle_of_pos = if angle_of_pos < 0. { angle_of_pos + 2.0 * PI } else { angle_of_pos };
        let is_active = selected.is_none()
            && !(pos.x == 0.0 && pos.y == 0.0)
            && angle_min < angle_of_pos
            && angle_max > angle_of_pos;

        unsafe {
            draw_slice(
                ui,
                txt,
                angle_base,
                (angle_min, angle_max),
                (radius_min, radius_max),
                is_active,
            )
        };

        if is_active {
            selected = Some(index);
        }
    }
    selected
}
