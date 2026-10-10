-- NEWS — fully declarative.
local SCENE = [==[
(
    items: [
        Header(title: "News", glyph: "\u{f1ea}", meta: "{news_label}"),
        Strip(chips: "news_chips", sel: "news_cat_sel", scroll: "news_cat_scroll",
              x: 0.0, y: 30.0, w: 0.0, h: 26.0, pad: 12.0, spacing: 6.0, chip_pad: 11.0, fs: 8.5,
              key_base: 13000),
        Text(x: 0.0, y: -6.0, center_x: true, center_y: true, text: "{news_msg}",
             font_size: 9.0, color: fg3, visible: "news_empty"),
        Rows(name: "news_rows", y: 59.0, row_h: 30.0, pitch: 30.0, pad: 12.0, row_dy: 2.0,
             cols: [
                 (x: 5.0, dy: 2.0, size: 9.5, truncate: 46.0, color: fg, hover: acc),
                 (x: 0.0, edge: 12.0, right: true, dy: 2.0, size: 8.0, icon: true, color: fg3, hover: acc),
                 (x: 0.0, edge: 12.0, right: true, dy: 14.0, size: 7.5, color: fg3),
             ],
             key_base: 13300, start: "news_scroll", hairline: true, hover_bar: true),
    ],
)
]==]
function draw(ctx) ctx.ui.scene(SCENE) end
