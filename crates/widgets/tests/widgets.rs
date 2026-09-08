//! Each widget is complete when its components are: what `div()`, `text()`
//! and `icon()` write at spawn, and what their handlers rewrite.

use app::prelude::*;
use assets::{AtlasId, BakedFont, GlyphInfo, SpriteRegion};
use layout::prelude::*;
use paint::prelude::*;
use utils::{Point, Size};
use widgets::prelude::*;

// ── fixtures ─────────────────────────────────────────────────────────────────

const RED: Color = Color::rgb(1.0, 0.0, 0.0);
const BLUE: Color = Color::rgb(0.0, 0.0, 1.0);
const BG: Color = Color::rgb(0.1, 0.1, 0.12);

/// A monospace font: every printable glyph but space is a 6x10 bitmap at
/// `(index * 10, 0)`, bearing (1, -2), advance 8; space advances 8 and has
/// no bitmap.
const fn glyphs() -> [GlyphInfo; 95] {
    let blank = GlyphInfo {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
        bearing_x: 0.0,
        bearing_y: 0.0,
        advance: 8.0,
    };
    let mut g = [blank; 95];
    let mut i = 1;
    while i < 95 {
        g[i] = GlyphInfo {
            x: i as f32 * 10.0,
            y: 0.0,
            w: 6.0,
            h: 10.0,
            bearing_x: 1.0,
            bearing_y: -2.0,
            advance: 8.0,
        };
        i += 1;
    }
    g
}

static FONT: BakedFont = BakedFont {
    atlas_id: AtlasId(7),
    size: 12.0,
    line_height: 16.0,
    ascent: 12.0,
    glyphs: glyphs(),
};

fn app() -> App {
    let mut app = App::new();
    app.add_module(LayoutModule).add_module(PaintModule);
    app
}

fn style(app: &App, id: impl Into<NodeId>) -> LayoutStyle {
    app.components::<LayoutStyle>().unwrap()[id].clone()
}

fn paint(app: &App, id: impl Into<NodeId>) -> Paint {
    app.components::<Paint>().unwrap()[id].clone()
}

fn measure(app: &App, id: impl Into<NodeId>) -> Measure {
    app.components::<Measure>().unwrap()[id]
}

fn sprites(app: &App, id: impl Into<NodeId>) -> Vec<MonochromeSprite> {
    match paint(app, id) {
        Paint::Sprites(run) => run,
        other => panic!("expected sprites, got {other:?}"),
    }
}

/// A glyph of the test font at pen position `pen`, index `i`, tinted `color`.
fn glyph(i: u32, pen: f32, color: Color) -> MonochromeSprite {
    MonochromeSprite {
        atlas: AtlasId(7),
        region: SpriteRegion {
            x: i as f32 * 10.0,
            y: 0.0,
            w: 6.0,
            h: 10.0,
        },
        offset: Point::new(pen + 1.0, 4.0),
        size: Size::new(6.0, 10.0),
        color,
    }
}

const A: u32 = b'a' as u32 - 32;
const B: u32 = b'b' as u32 - 32;
const C: u32 = b'c' as u32 - 32;

// ── div ──────────────────────────────────────────────────────────────────────

#[test]
fn div_writes_its_style_and_quad() {
    let mut app = app();
    let d = app
        .spawn(
            app.root(),
            div()
                .size(px(48.0), px(48.0))
                .row()
                .gap(px(4.0))
                .color(RED)
                .radius(4.0),
        )
        .unwrap();
    let s = style(&app, d);
    assert_eq!((s.width, s.height), (px(48.0), px(48.0)));
    assert_eq!(s.row_gap, px(4.0));
    assert_eq!(paint(&app, d), Paint::Quad(Quad::new(RED).radius(4.0)));
}

#[test]
fn a_div_with_no_colour_is_a_quad_that_draws_nothing() {
    let mut app = app();
    let d = app.spawn(app.root(), div()).unwrap();
    assert_eq!(style(&app, d), LayoutStyle::default());
    assert_eq!(paint(&app, d), Paint::Quad(Quad::default()));
    assert!(paint(&app, d).is_invisible());
}

#[test]
fn div_border_writes_the_quad_and_the_style_together() {
    let mut app = app();
    let d = app
        .spawn(app.root(), div().color(RED).border(2.0, BG))
        .unwrap();
    assert_eq!(style(&app, d).border, Edges::all(px(2.0)));
    assert_eq!(paint(&app, d), Paint::Quad(Quad::new(RED).border(2.0, BG)));
}

#[test]
fn div_takes_whole_values_too() {
    let mut app = app();
    let layout = LayoutStyle::default()
        .column()
        .center()
        .padding_all(px(3.0));
    let quad = Quad::new(BLUE).radius(9.0);
    let d = app
        .spawn(app.root(), div().layout(layout.clone()).paint(quad))
        .unwrap();
    assert_eq!(style(&app, d), layout);
    assert_eq!(paint(&app, d), Paint::Quad(quad));
}

#[test]
fn set_layout_replaces_a_divs_style() {
    let mut app = app();
    let d = app.spawn(app.root(), div().size(px(1.0), px(1.0))).unwrap();
    let wide = LayoutStyle::default().size(percent(50.0), px(20.0)).wrap();
    app.emit(SetLayout(wide.clone()), &[d.id()]);
    app.flush();
    assert_eq!(style(&app, d), wide);
}

#[test]
fn set_quad_replaces_the_quad_and_keeps_the_style_border_in_step() {
    let mut app = app();
    let d = app
        .spawn(
            app.root(),
            div().size(px(10.0), px(10.0)).color(RED).border(2.0, BG),
        )
        .unwrap();

    app.emit(SetQuad(Quad::new(BLUE).border(3.0, RED)), &[d.id()]);
    app.flush();
    assert_eq!(
        paint(&app, d),
        Paint::Quad(Quad::new(BLUE).border(3.0, RED))
    );
    let s = style(&app, d);
    assert_eq!(s.border, Edges::all(px(3.0)));
    assert_eq!(s.width, px(10.0), "the rest of the style is untouched");

    app.emit(SetQuad(Quad::new(BLUE)), &[d.id()]);
    app.flush();
    assert_eq!(
        style(&app, d).border,
        Edges::all(px(0.0)),
        "no border takes no space"
    );
}

// ── text ─────────────────────────────────────────────────────────────────────

#[test]
fn text_measures_its_run_and_paints_one_sprite_per_glyph() {
    let mut app = app();
    let t = app
        .spawn(app.root(), text("ab").font(&FONT).color(RED))
        .unwrap();
    let w = app.widget::<Text>(t).unwrap();
    assert_eq!(w.text, "ab");
    assert_eq!(w.color, RED);
    assert_eq!(measure(&app, t), Measure(Some(Size::new(16.0, 16.0))));
    assert_eq!(sprites(&app, t), [glyph(A, 0.0, RED), glyph(B, 8.0, RED)]);
    assert_eq!(style(&app, t), LayoutStyle::default());
}

#[test]
fn text_glyphs_sit_on_the_baseline() {
    // ascent 12, bearing_y -2 (bitmap bottom 2px below the baseline),
    // height 10: the bitmap's top is 4px below the content box's top.
    let t = Text {
        font: Some(&FONT),
        text: "a".into(),
        color: RED,
    };
    let Paint::Sprites(run) = t.paint() else {
        panic!()
    };
    assert_eq!(run[0].offset, Point::new(1.0, 4.0));
    assert_eq!(run[0].size, Size::new(6.0, 10.0));
}

#[test]
fn text_without_a_font_measures_nothing_and_draws_nothing() {
    let mut app = app();
    let t = app.spawn(app.root(), text("ab")).unwrap();
    assert_eq!(measure(&app, t), Measure(None));
    assert_eq!(paint(&app, t), Paint::None);
    assert_eq!(app.widget::<Text>(t).unwrap().color, Color::WHITE);
}

#[test]
fn spaces_advance_without_a_sprite_and_unprintables_are_skipped() {
    let mut app = app();
    let t = app.spawn(app.root(), text("a \nb").font(&FONT)).unwrap();
    assert_eq!(measure(&app, t), Measure(Some(Size::new(24.0, 16.0))));
    assert_eq!(
        sprites(&app, t),
        [glyph(A, 0.0, Color::WHITE), glyph(B, 16.0, Color::WHITE)]
    );
}

#[test]
fn an_empty_string_with_a_font_is_an_empty_run() {
    let mut app = app();
    let t = app.spawn(app.root(), text("").font(&FONT)).unwrap();
    assert_eq!(measure(&app, t), Measure(Some(Size::new(0.0, 16.0))));
    assert_eq!(paint(&app, t), Paint::Sprites(Vec::new()));
}

#[test]
fn set_text_set_font_and_set_color_rewrite_measure_and_paint() {
    let mut app = app();
    let t = app.spawn(app.root(), text("a")).unwrap();

    app.emit(SetFont(&FONT), &[t.id()]);
    app.flush();
    assert_eq!(measure(&app, t), Measure(Some(Size::new(8.0, 16.0))));
    assert_eq!(sprites(&app, t), [glyph(A, 0.0, Color::WHITE)]);

    app.emit(SetText("abc".into()), &[t.id()]);
    app.flush();
    assert_eq!(app.widget::<Text>(t).unwrap().text, "abc");
    assert_eq!(measure(&app, t), Measure(Some(Size::new(24.0, 16.0))));
    assert_eq!(sprites(&app, t).len(), 3);

    app.emit(SetColor(BLUE), &[t.id()]);
    app.flush();
    assert_eq!(
        sprites(&app, t),
        [
            glyph(A, 0.0, BLUE),
            glyph(B, 8.0, BLUE),
            glyph(C, 16.0, BLUE)
        ]
    );
    assert_eq!(
        measure(&app, t),
        Measure(Some(Size::new(24.0, 16.0))),
        "a colour moves nothing"
    );
}

#[test]
fn text_is_laid_out_at_its_measure() {
    let mut app = app();
    let row = app
        .spawn(
            app.root(),
            div().absolute().size(px(200.0), px(100.0)).row().center(),
        )
        .unwrap();
    let t = app.spawn(row, text("abc").font(&FONT)).unwrap();
    app.signal(Tick);
    app.flush();
    let rect = app.components::<Layout>().unwrap()[t].rect;
    assert_eq!(rect.size, Size::new(24.0, 16.0));
    assert_eq!(rect.origin, Point::new(88.0, 42.0), "centred in the row");
}

// ── icon ─────────────────────────────────────────────────────────────────────

fn a_sprite() -> Sprite {
    Sprite {
        atlas: AtlasId(3),
        region: SpriteRegion {
            x: 40.0,
            y: 8.0,
            w: 24.0,
            h: 20.0,
        },
    }
}

#[test]
fn icon_measures_its_region_and_paints_one_sprite() {
    let mut app = app();
    let i = app
        .spawn(app.root(), icon().sprite(a_sprite()).color(RED))
        .unwrap();
    assert_eq!(app.widget::<Icon>(i).unwrap().sprite, Some(a_sprite()));
    assert_eq!(measure(&app, i), Measure(Some(Size::new(24.0, 20.0))));
    assert_eq!(
        sprites(&app, i),
        [MonochromeSprite {
            atlas: AtlasId(3),
            region: a_sprite().region,
            offset: Point::ZERO,
            size: Size::new(24.0, 20.0),
            color: RED,
        }]
    );
}

#[test]
fn icon_without_a_sprite_measures_nothing_and_draws_nothing() {
    let mut app = app();
    let i = app.spawn(app.root(), icon()).unwrap();
    assert_eq!(measure(&app, i), Measure(None));
    assert_eq!(paint(&app, i), Paint::None);
}

#[test]
fn set_sprite_and_set_color_rewrite_measure_and_paint() {
    let mut app = app();
    let i = app.spawn(app.root(), icon()).unwrap();

    app.emit(SetSprite(a_sprite()), &[i.id()]);
    app.flush();
    assert_eq!(measure(&app, i), Measure(Some(Size::new(24.0, 20.0))));
    assert_eq!(sprites(&app, i)[0].color, Color::WHITE);

    app.emit(SetColor(BLUE), &[i.id()]);
    app.flush();
    assert_eq!(sprites(&app, i)[0].color, BLUE);
    assert_eq!(sprites(&app, i)[0].size, Size::new(24.0, 20.0));
}

#[test]
fn icon_takes_a_whole_style() {
    let mut app = app();
    let layout = LayoutStyle::default().margin(Edges::all(px(4.0)));
    let i = app
        .spawn(app.root(), icon().layout(layout.clone()))
        .unwrap();
    assert_eq!(style(&app, i), layout);
}
