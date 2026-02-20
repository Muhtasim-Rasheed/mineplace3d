use std::rc::Rc;

use glam::{Vec2, Vec4};

use crate::{
    abs::TextureHandle,
    render::ui::widgets::{Font, Label, NineSlice, Stack, Widget},
};

pub struct InputField {
    pub position: Vec2,
    pub size: Vec2,
    pub text: String,
    pub label_color: Vec4,
    pub label_font_size: f32,
    pub cursor_pos: usize,
    pub placeholder: String,
    pub sanitize: Option<String>,
    hovered: bool,
    hover_last: bool,
    focused: bool,
    stack: Stack,
    texture: TextureHandle,
    font: Rc<Font>,
}

impl InputField {
    pub fn new(
        placeholder: &str,
        label_color: Vec4,
        label_font_size: f32,
        size: Vec2,
        sanitize: Option<&str>,
        font: &Rc<Font>,
        texture: TextureHandle,
    ) -> Self {
        let stack = Stack::new(super::Alignment::Start, super::Alignment::Center, 0.0);
        let mut inputfield = Self {
            position: Vec2::ZERO,
            size,
            text: String::new(),
            label_color,
            label_font_size,
            cursor_pos: 0,
            placeholder: placeholder.to_string(),
            sanitize: sanitize.map(|s| s.to_string()),
            hovered: false,
            hover_last: false,
            focused: false,
            stack,
            texture,
            font: Rc::clone(font),
        };

        inputfield.setup_stack();

        inputfield
    }

    fn setup_stack(&mut self) {
        self.stack = Stack::new(super::Alignment::Start, super::Alignment::Center, 0.0);
        self.stack.add_widget(NineSlice::new(
            self.texture,
            glam::uvec2(32, 0),
            glam::uvec2(16, 16),
            self.size,
            glam::uvec4(6, 6, 4, 4),
            4,
            if self.hovered && !self.focused {
                Vec4::new(1.2, 1.2, 1.2, 1.0)
            } else {
                Vec4::ONE
            },
        ));
        if self.text.is_empty() && !self.focused {
            self.stack.add_widget(Label::new(
                &format!("  {}", self.placeholder),
                self.label_font_size,
                self.label_color * Vec4::new(1.0, 1.0, 1.0, 0.5),
                &self.font,
            ));
        } else {
            self.stack.add_widget(Label::new(
                &format!("  {}", self.text),
                self.label_font_size,
                self.label_color,
                &self.font,
            ));
        }
    }

    fn update_stack(&mut self) {
        if let Some(nine_slice) = self.stack.get_widget_mut::<NineSlice>(0) {
            nine_slice.tint = if self.hovered && !self.focused {
                Vec4::new(1.2, 1.2, 1.2, 1.0)
            } else {
                Vec4::ONE
            };
            nine_slice.position = self.position;
            nine_slice.size = self.size;
        } else {
            self.setup_stack();
        }
        if let Some(label) = self.stack.get_widget_mut::<Label>(1) {
            if self.text.is_empty() && !self.focused {
                label.text = format!("  {}", self.placeholder);
                label.color = self.label_color * Vec4::new(1.0, 1.0, 1.0, 0.5);
            } else {
                label.text = format!("  {}", self.text);
                label.color = self.label_color;
            }
            label.font_size = self.label_font_size;
        } else {
            self.setup_stack();
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered
    }
}

impl Widget for InputField {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn size_hint(&self) -> Vec2 {
        self.size
    }

    fn update(&mut self, ctx: &crate::other::UpdateContext) {
        self.hover_last = self.hovered;
        let mouse_pos = ctx.mouse.position;
        let mouse_pressed = ctx.mouse.down.contains(&sdl2::mouse::MouseButton::Left);
        self.hovered = mouse_pos.x >= self.position.x
            && mouse_pos.x <= self.position.x + self.size.x
            && mouse_pos.y >= self.position.y
            && mouse_pos.y <= self.position.y + self.size.y;
        if mouse_pressed {
            self.focused = self.hovered;
        }
        if self.focused {
            let repeated = &ctx.keyboard.repeated;
            if repeated.contains(&sdl2::keyboard::Keycode::Backspace) {
                if self.cursor_pos > 0 {
                    self.text.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            } else if repeated.contains(&sdl2::keyboard::Keycode::Left) {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            } else if repeated.contains(&sdl2::keyboard::Keycode::Right) {
                if self.cursor_pos < self.text.len() {
                    self.cursor_pos += 1;
                }
            } else if repeated.contains(&sdl2::keyboard::Keycode::Home) {
                self.cursor_pos = 0;
            } else if repeated.contains(&sdl2::keyboard::Keycode::End) {
                self.cursor_pos = self.text.len();
            } else {
                let sanitized_input = if let Some(sanitize) = &self.sanitize {
                    ctx.keyboard
                        .text_input
                        .chars()
                        .map(|c| if sanitize.contains(c) { '_' } else { c })
                        .collect::<String>()
                } else {
                    ctx.keyboard.text_input.clone()
                };
                self.text.insert_str(self.cursor_pos, &sanitized_input);
                self.cursor_pos += sanitized_input.len();
            }
        }
        self.update_stack();
    }

    fn layout(&mut self, ctx: &super::LayoutContext) -> Vec2 {
        let measured_size = self.size_hint().min(ctx.max_size);
        self.position = ctx.cursor;
        let layout_ctx = super::LayoutContext {
            max_size: measured_size,
            cursor: self.position,
        };
        self.stack.layout(&layout_ctx);
        Vec2::new(
            measured_size.x.min(ctx.max_size.x),
            measured_size.y.min(ctx.max_size.y),
        )
    }

    fn draw(&self, ui_renderer: &mut crate::render::ui::uirenderer::UIRenderer) {
        self.stack.draw(ui_renderer);
        // Draw cursor
        if self.focused {
            let cursor_x = self.position.x
                + self
                    .font
                    .measure_text(
                        &format!("  {}", &self.text[..self.cursor_pos]),
                        self.label_font_size,
                    )
                    .x;
            let cursor_y = self.position.y + (self.size.y - self.label_font_size) / 2.0;
            ui_renderer.add_command(crate::render::ui::uirenderer::DrawCommand {
                rect: [
                    Vec2::new(cursor_x, cursor_y),
                    Vec2::new(cursor_x + 2.0, cursor_y + self.label_font_size),
                ],
                uv_rect: [Vec2::ZERO, Vec2::ONE],
                mode: crate::render::ui::uirenderer::UIRenderMode::Color(Vec4::ONE),
            });
        }
    }
}
