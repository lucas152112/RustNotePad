use std::fmt::Write as _;
use std::io::Write as _;

use image::{codecs::png::PngEncoder, ColorType, ImageBuffer, ImageEncoder, Rgba};
use thiserror::Error;

use crate::display::{Color, DisplayCommand, Point, PrintDisplayList, Stroke};
use crate::job::{Margin, Orientation, PrintJobControllerState, PrintJobOptions};
use crate::layout::{
    LayoutInput, LayoutOptions, LayoutSummary, PageLayout, PaginationResult, Paginator,
};
use crate::platform::{PlatformAdapter, PlatformJobHandle, SpoolPage};
use crate::preview::{PreviewCache, PreviewEntry, PrintPreviewKey};
use crate::template::HeaderFooterContext;

/// Result produced after executing a print job.
/// 列印作業完成後所產生的結果。
#[derive(Debug, Clone)]
pub struct PrintJobResult {
    pub summary: LayoutSummary,
    pub pdf_data: Vec<u8>,
    pub state: PrintJobControllerState,
}

/// Configuration for preview generation.
/// 建立預覽時所需的設定資訊。
#[derive(Debug)]
pub struct PreviewConfig<'a> {
    pub cache: &'a mut PreviewCache,
    pub zoom_levels: &'a [u32],
    pub base_dpi: u32,
}

/// Errors raised while running the print pipeline.
/// 列印管線執行時可能發生的錯誤。
#[derive(Debug, Error)]
pub enum PrintJobError {
    #[error("layout failed: {0}")]
    Layout(String),
    #[error("preview rendering failed: {0}")]
    Preview(String),
    #[error("PDF generation failed: {0}")]
    Pdf(String),
    #[error("platform adapter failed: {0}")]
    Platform(String),
}

/// Executes the print pipeline end-to-end, producing previews, PDF, and spooled pages.
/// 端到端執行列印管線，產生預覽、PDF 與待送佇列的頁面。
pub fn run_print_job<P, A>(
    paginator: &P,
    input: &dyn LayoutInput,
    layout_options: &LayoutOptions,
    job_options: &PrintJobOptions,
    adapter: &A,
    preview: Option<PreviewConfig<'_>>,
) -> Result<PrintJobResult, PrintJobError>
where
    P: Paginator,
    A: PlatformAdapter,
    A::Error: std::fmt::Display,
{
    let pagination = paginator.paginate(input, layout_options);
    let summary = pagination.summary.clone();
    let pages = augment_pages_with_templates(
        pagination,
        job_options,
        layout_options.paper,
        layout_options.orientation,
    );

    let (page_width, page_height) = job_options.paper.to_points(job_options.orientation);

    if let Some(mut preview_config) = preview {
        for page in &pages {
            if let Err(err) = cache_previews_for_page(
                &mut preview_config,
                job_options,
                page,
                page_width,
                page_height,
            ) {
                return Err(PrintJobError::Preview(err));
            }
        }
    }

    let pdf_data = match render_pdf_document(&pages, page_width, page_height) {
        Ok(data) => data,
        Err(err) => return Err(PrintJobError::Pdf(err)),
    };

    match spool_pages(adapter, job_options, &pages) {
        Ok(()) => {}
        Err(err) => {
            return Err(PrintJobError::Platform(err));
        }
    }

    Ok(PrintJobResult {
        summary,
        pdf_data,
        state: PrintJobControllerState::Completed,
    })
}

fn augment_pages_with_templates(
    pagination: PaginationResult,
    job_options: &PrintJobOptions,
    paper: crate::job::PaperSize,
    orientation: Orientation,
) -> Vec<PageLayout> {
    let total_pages = pagination.summary.total_pages.max(1);
    let mut output = Vec::with_capacity(pagination.pages.len());
    let (page_width, page_height) = paper.to_points(orientation);

    for page in pagination.pages {
        let mut display_list = page.display_list.clone();
        append_header_footer(
            &mut display_list,
            page.page_number,
            total_pages,
            job_options,
            page_width,
            page_height,
        );
        output.push(PageLayout::new(
            page.page_number,
            page.line_ranges.clone(),
            display_list,
        ));
    }

    output
}

fn append_header_footer(
    display_list: &mut PrintDisplayList,
    page_number: u32,
    total_pages: u32,
    job_options: &PrintJobOptions,
    page_width: f32,
    page_height: f32,
) {
    let header_context = HeaderFooterContext {
        page_number,
        page_count: Some(total_pages),
        ..Default::default()
    };
    let footer_context = header_context.clone();
    let header = job_options.header_template.render(&header_context);
    let footer = job_options.footer_template.render(&footer_context);

    let header_y = job_options.margins.top / 2.0;
    let footer_y = page_height - job_options.margins.bottom / 2.0;

    let font_family = "Helvetica".to_string();
    let header_font_size = 9.0;
    let footer_font_size = 9.0;

    push_template_text(
        display_list,
        &header.left,
        job_options.margins.left,
        header_y,
        header_font_size,
        font_family.clone(),
        Color::new(0.2, 0.2, 0.2, 1.0),
    );
    push_template_text(
        display_list,
        &header.center,
        (page_width - estimate_text_width(&header.center, header_font_size)) / 2.0,
        header_y,
        header_font_size,
        font_family.clone(),
        Color::new(0.2, 0.2, 0.2, 1.0),
    );
    push_template_text(
        display_list,
        &header.right,
        page_width
            - job_options.margins.right
            - estimate_text_width(&header.right, header_font_size),
        header_y,
        header_font_size,
        font_family.clone(),
        Color::new(0.2, 0.2, 0.2, 1.0),
    );

    push_template_text(
        display_list,
        &footer.left,
        job_options.margins.left,
        footer_y,
        footer_font_size,
        font_family.clone(),
        Color::new(0.2, 0.2, 0.2, 1.0),
    );
    push_template_text(
        display_list,
        &footer.center,
        (page_width - estimate_text_width(&footer.center, footer_font_size)) / 2.0,
        footer_y,
        footer_font_size,
        font_family.clone(),
        Color::new(0.2, 0.2, 0.2, 1.0),
    );
    push_template_text(
        display_list,
        &footer.right,
        page_width
            - job_options.margins.right
            - estimate_text_width(&footer.right, footer_font_size),
        footer_y,
        footer_font_size,
        font_family.clone(),
        Color::new(0.2, 0.2, 0.2, 1.0),
    );

    let header_rule_y = job_options.margins.top.max(2.0);
    let footer_rule_y = page_height - job_options.margins.bottom.max(2.0);

    display_list.push(DisplayCommand::HorizontalRule {
        start: Point {
            x: job_options.margins.left,
            y: header_rule_y,
        },
        end: Point {
            x: page_width - job_options.margins.right,
            y: header_rule_y,
        },
        stroke: Stroke {
            width: 0.5,
            color: Color::new(0.6, 0.6, 0.6, 1.0),
        },
    });

    display_list.push(DisplayCommand::HorizontalRule {
        start: Point {
            x: job_options.margins.left,
            y: footer_rule_y,
        },
        end: Point {
            x: page_width - job_options.margins.right,
            y: footer_rule_y,
        },
        stroke: Stroke {
            width: 0.5,
            color: Color::new(0.6, 0.6, 0.6, 1.0),
        },
    });
}

fn push_template_text(
    display_list: &mut PrintDisplayList,
    text: &str,
    x: f32,
    y: f32,
    size: f32,
    font_family: String,
    color: Color,
) {
    if text.is_empty() {
        return;
    }
    display_list.push(DisplayCommand::GlyphRun(crate::display::GlyphRun {
        text: text.to_string(),
        font_family,
        font_size_pt: size,
        position: Point { x, y },
        color,
        background: None,
    }));
}

fn cache_previews_for_page(
    config: &mut PreviewConfig<'_>,
    job_options: &PrintJobOptions,
    page: &PageLayout,
    page_width: f32,
    page_height: f32,
) -> Result<(), String> {
    for zoom in config.zoom_levels {
        let entry = render_preview_png(
            &page.display_list,
            page_width,
            page_height,
            *zoom,
            config.base_dpi,
            job_options.margins,
        )
        .map_err(|err| err)?;
        let key = PrintPreviewKey {
            job_id: job_options.job_id,
            page: page.page_number,
            zoom_percent: *zoom,
        };
        config.cache.insert(key, entry);
    }
    Ok(())
}

fn render_preview_png(
    display_list: &PrintDisplayList,
    page_width_pt: f32,
    page_height_pt: f32,
    zoom_percent: u32,
    base_dpi: u32,
    margins: Margin,
) -> Result<PreviewEntry, String> {
    let dpi = ((base_dpi as u64 * zoom_percent as u64) / 100).max(72) as u32;
    let scale = dpi as f32 / 72.0;
    let width_px = (page_width_pt * scale).ceil().max(1.0) as u32;
    let height_px = (page_height_pt * scale).ceil().max(1.0) as u32;

    let mut image = ImageBuffer::from_pixel(width_px, height_px, Rgba([255, 255, 255, 255]));

    for command in &display_list.commands {
        match command {
            DisplayCommand::BackgroundRect(rect) => {
                let x = (rect.origin.x * scale).round() as i32;
                let y = (rect.origin.y * scale).round() as i32;
                let w = (rect.size.width * scale).ceil() as i32;
                let h = (rect.size.height * scale).ceil() as i32;
                fill_rect(&mut image, x, y, w, h, rect.color);
            }
            DisplayCommand::GlyphRun(run) => {
                let estimated_width = estimate_text_run_width(run);
                let glyph_height = (run.font_size_pt * 1.1).max(8.0);
                let x = (run.position.x * scale).round() as i32;
                let y = (run.position.y * scale).round() as i32;
                let w = (estimated_width * scale).ceil() as i32;
                let h = (glyph_height * scale).ceil() as i32;
                fill_rect(&mut image, x, y, w.max(2), h.max(2), run.color);
            }
            DisplayCommand::HorizontalRule { start, end, stroke } => {
                let x0 = (start.x * scale).round() as i32;
                let x1 = (end.x * scale).round() as i32;
                let y = (start.y * scale).round() as i32;
                draw_horizontal_line(&mut image, x0, x1, y, stroke);
            }
        }
    }

    // Render margin guides for preview context.
    // 為預覽畫面繪製邊界輔助線。
    draw_margin_guides(&mut image, margins, scale, Color::new(0.9, 0.9, 0.9, 1.0));

    let mut data = Vec::new();
    PngEncoder::new(&mut data)
        .write_image(image.as_raw(), width_px, height_px, ColorType::Rgba8)
        .map_err(|err| err.to_string())?;

    Ok(PreviewEntry {
        width_px,
        height_px,
        dpi,
        data,
    })
}

fn fill_rect(
    buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    color: Color,
) {
    if width <= 0 || height <= 0 {
        return;
    }
    let width_px = buffer.width() as i32;
    let height_px = buffer.height() as i32;
    let x0 = x.clamp(0, width_px);
    let y0 = y.clamp(0, height_px);
    let x1 = (x + width).clamp(0, width_px);
    let y1 = (y + height).clamp(0, height_px);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    let rgba = color_to_rgba(color);
    for yy in y0..y1 {
        for xx in x0..x1 {
            buffer.put_pixel(xx as u32, yy as u32, rgba);
        }
    }
}

fn draw_horizontal_line(
    buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x0: i32,
    x1: i32,
    y: i32,
    stroke: &Stroke,
) {
    if y < 0 || y >= buffer.height() as i32 {
        return;
    }
    let start = x0.min(x1).max(0) as u32;
    let end = x0.max(x1).min(buffer.width() as i32) as u32;
    let rgba = color_to_rgba(stroke.color);
    for xx in start..end {
        buffer.put_pixel(xx, y as u32, rgba);
    }
}

fn draw_margin_guides(
    buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    margins: Margin,
    scale: f32,
    color: Color,
) {
    let left = (margins.left * scale).round() as i32;
    let right = buffer.width() as i32 - (margins.right * scale).round() as i32;
    let top = (margins.top * scale).round() as i32;
    let bottom = buffer.height() as i32 - (margins.bottom * scale).round() as i32;
    draw_vertical_band(buffer, left, color);
    draw_vertical_band(buffer, right, color);
    draw_horizontal_band(buffer, top, color);
    draw_horizontal_band(buffer, bottom, color);
}

fn draw_vertical_band(buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: i32, color: Color) {
    if x < 0 || x >= buffer.width() as i32 {
        return;
    }
    let rgba = color_to_rgba(color);
    for y in 0..buffer.height() {
        buffer.put_pixel(x as u32, y, rgba);
    }
}

fn draw_horizontal_band(buffer: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, y: i32, color: Color) {
    if y < 0 || y >= buffer.height() as i32 {
        return;
    }
    let rgba = color_to_rgba(color);
    for x in 0..buffer.width() {
        buffer.put_pixel(x, y as u32, rgba);
    }
}

fn color_to_rgba(color: Color) -> Rgba<u8> {
    Rgba([
        clamp_to_u8(color.r),
        clamp_to_u8(color.g),
        clamp_to_u8(color.b),
        clamp_to_u8(color.a),
    ])
}

fn clamp_to_u8(value: f32) -> u8 {
    (value.max(0.0).min(1.0) * 255.0).round() as u8
}

fn estimate_text_width(text: &str, font_size: f32) -> f32 {
    (text.chars().count() as f32) * font_size * 0.6
}

fn estimate_text_run_width(run: &crate::display::GlyphRun) -> f32 {
    estimate_text_width(&run.text, run.font_size_pt.max(1.0))
}

fn spool_pages<A>(
    adapter: &A,
    job_options: &PrintJobOptions,
    pages: &[PageLayout],
) -> Result<(), String>
where
    A: PlatformAdapter,
    A::Error: std::fmt::Display,
{
    let mut handle = adapter
        .begin_job(job_options)
        .map_err(|err| err.to_string())?;

    for page in pages {
        handle
            .submit_page(SpoolPage {
                job_id: job_options.job_id,
                page_number: page.page_number,
                display_list: page.display_list.clone(),
            })
            .map_err(|err| err.to_string())?;
    }

    handle.finish().map_err(|err| err.to_string())
}

fn render_pdf_document(
    pages: &[PageLayout],
    page_width: f32,
    page_height: f32,
) -> Result<Vec<u8>, String> {
    if pages.is_empty() {
        return Err("pagination produced no pages".to_string());
    }

    let mut builder = PdfBuilder::new();
    let font_object = builder.add_object("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
    let mut page_objects = Vec::new();

    for page in pages {
        let content_stream = render_page_stream(&page.display_list, page_height)?;
        let content_object = builder.add_stream(&content_stream);
        let page_object = builder.add_object(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {width} {height}] \
             /Resources << /Font << /F1 {font} 0 R >> >> /Contents {content} 0 R >>",
            width = fmt_float(page_width),
            height = fmt_float(page_height),
            font = font_object,
            content = content_object
        ));
        page_objects.push(page_object);
    }

    let kids = page_objects
        .iter()
        .map(|obj| format!("{obj} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    let pages_object = builder.add_object(format!(
        "<< /Type /Pages /Count {count} /Kids [{kids}] >>",
        count = pages.len()
    ));
    builder.set_catalog(format!(
        "<< /Type /Catalog /Pages {pages} 0 R >>",
        pages = pages_object
    ));

    Ok(builder.finish())
}

fn render_page_stream(
    display_list: &PrintDisplayList,
    page_height: f32,
) -> Result<Vec<u8>, String> {
    let mut stream = String::new();
    for command in &display_list.commands {
        match command {
            DisplayCommand::GlyphRun(run) => {
                let text = pdf_escape_text(&run.text);
                let x = fmt_float(run.position.x);
                let y = fmt_float(page_height - run.position.y - run.font_size_pt);
                let size = fmt_float(run.font_size_pt);
                writeln!(
                    &mut stream,
                    "{color} rg\nBT\n/F1 {size} Tf\n1 0 0 1 {x} {y} Tm\n({text}) Tj\nET",
                    color = rgb_to_pdf(run.color),
                    size = size,
                    x = x,
                    y = y,
                    text = text
                )
                .map_err(|err| err.to_string())?;
            }
            DisplayCommand::BackgroundRect(rect) => {
                let x = fmt_float(rect.origin.x);
                let y = fmt_float(page_height - rect.origin.y - rect.size.height);
                let w = fmt_float(rect.size.width);
                let h = fmt_float(rect.size.height);
                writeln!(
                    &mut stream,
                    "{color} rg\n{x} {y} {w} {h} re f",
                    color = rgb_to_pdf(rect.color),
                    x = x,
                    y = y,
                    w = w,
                    h = h
                )
                .map_err(|err| err.to_string())?;
            }
            DisplayCommand::HorizontalRule { start, end, stroke } => {
                let x0 = fmt_float(start.x);
                let x1 = fmt_float(end.x);
                let y = fmt_float(page_height - start.y);
                let width = fmt_float(stroke.width);
                writeln!(
                    &mut stream,
                    "{color} RG\n{width} w\n{x0} {y} m {x1} {y} l S",
                    color = rgb_to_pdf(stroke.color),
                    width = width,
                    x0 = x0,
                    x1 = x1,
                    y = y
                )
                .map_err(|err| err.to_string())?;
            }
        }
    }
    Ok(stream.into_bytes())
}

fn fmt_float(value: f32) -> String {
    format!("{:.3}", value)
}

fn rgb_to_pdf(color: Color) -> String {
    format!(
        "{:.3} {:.3} {:.3}",
        color.r.clamp(0.0, 1.0),
        color.g.clamp(0.0, 1.0),
        color.b.clamp(0.0, 1.0)
    )
}

fn pdf_escape_text(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '(' | ')' | '\\' => {
                output.push('\\');
                output.push(ch);
            }
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            _ => output.push(ch),
        }
    }
    output
}

struct PdfBuilder {
    objects: Vec<PdfObject>,
    catalog: Option<String>,
}

impl PdfBuilder {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            catalog: None,
        }
    }

    fn add_object(&mut self, body: impl Into<String>) -> usize {
        let number = self.objects.len() + 1;
        self.objects.push(PdfObject {
            number,
            body: body.into(),
        });
        number
    }

    fn add_stream(&mut self, stream: &[u8]) -> usize {
        let mut body = format!("<< /Length {} >>\nstream\n", stream.len());
        body.push_str(&String::from_utf8_lossy(stream));
        body.push_str("\nendstream");
        self.add_object(body)
    }

    fn set_catalog(&mut self, catalog: String) {
        self.catalog = Some(catalog);
    }

    fn finish(mut self) -> Vec<u8> {
        if let Some(catalog) = self.catalog.take() {
            self.add_object(catalog);
        }

        let mut output = Vec::new();
        output.extend_from_slice(b"%PDF-1.4\n%\xFF\xFF\xFF\xFF\n");
        let mut offsets = Vec::with_capacity(self.objects.len() + 1);
        offsets.push(0);

        for object in &self.objects {
            offsets.push(output.len());
            writeln!(
                &mut output,
                "{} 0 obj\n{}\nendobj",
                object.number, object.body
            )
            .expect("write pdf object");
        }

        let xref_start = output.len();
        writeln!(
            &mut output,
            "xref\n0 {}\n0000000000 65535 f ",
            self.objects.len() + 1
        )
        .expect("write xref header");
        for offset in offsets.iter().skip(1) {
            writeln!(&mut output, "{:010} 00000 n ", offset).expect("write xref entry");
        }

        writeln!(
            &mut output,
            "trailer\n<< /Size {} /Root {} 0 R >>",
            self.objects.len() + 1,
            self.objects.len()
        )
        .expect("write trailer");
        writeln!(&mut output, "startxref\n{}\n%%EOF", xref_start).expect("write footer");

        output
    }
}

struct PdfObject {
    number: usize,
    body: String,
}
