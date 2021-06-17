use wgpu_glyph::{ab_glyph, GlyphBrushBuilder, Section, Text};

pub struct GameText {
    glyph_brush: wgpu_glyph::GlyphBrush<()>,
}

impl GameText {
    pub fn new(font_path: &'static str, device: &wgpu::Device) -> Self {
        let font = ab_glyph::FontArc::try_from_slice(font_path.as_bytes()).unwrap();
        // let font = ab_glyph::FontArc::try_from_slice(include_bytes!(font_path.as_bytes())).unwrap();
        let glyph_brush =
            GlyphBrushBuilder::using_font(font).build(&device, wgpu::TextureFormat::Bgra8UnormSrgb);
        Self { glyph_brush }
    }

    pub fn queue(&mut self, text: &str, pos: (f32, f32), color: [f32; 4], scale: f32) {
        let text = Text::new(text).with_color(color).with_scale(scale);
        self.glyph_brush.queue(Section {
            screen_position: pos,
            // bounds: (size.width as f32, size.height as f32),
            text: vec![text],
            ..Section::default()
        });
    }

    pub fn render_queued(
        &mut self,
        device: &wgpu::Device,
        staging_belt: &mut wgpu::util::StagingBelt,
        encoder: &mut wgpu::CommandEncoder,
        frame_view: &wgpu::TextureView,
        size: winit::dpi::PhysicalSize<u32>,
    ) {
        self.glyph_brush
            .draw_queued(
                device,
                staging_belt,
                encoder,
                frame_view,
                size.width,
                size.height,
            )
            .expect("Draw queued");
    }
}
