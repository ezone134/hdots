-- Quote card — fully declarative (zero `Ink`). Embeds the scene.
local SCENE = [==[
(
    // quote card — fully declarative (zero `Ink`).
    items: [
        Header(title: "Quote", glyph: "\u{f10d}", glyph_color: acc, pad: 12.0),
        Text(x: 12.0, y: 9.0, w: 0.0, right: true, text: "{quote_meta}", font_size: 8.5,
             font: ui, weight: regular, color: value("quote_meta_ink")),
        TextWrap(text: "{quote_text}", caption: "— {quote_author}", top: 28.0, bottom: 40.0,
                 font_size: 10.0, line_h: 13.0, color: fg, visible: "quote_has"),
        Text(x: 0.0, y: -6.0, w: 0.0, center_x: true, center_y: true,
             text: "fetching a quote…", font_size: 9.0, font: ui, weight: regular,
             color: fg3, visible: "quote_fetching"),
        Text(x: 0.0, y: -6.0, w: 0.0, center_x: true, center_y: true,
             text: "no quote yet", font_size: 9.0, font: ui, weight: regular,
             color: fg3, visible: "quote_empty"),
        Row(x: 0.0, y: 8.0, w: 0.0, h: 26.0, bottom: true, pad: 0.0, spacing: 12.0, halign: center,
            items: [
                Stack(x: 0.0, y: 0.0, w: 88.0, h: 26.0, pad: 0.0,
                      items: [
                          Surface(x: 0.0, y: 0.0, w: 88.0, h: 26.0, r: 13.0, color: value("quote_new_fill")),
                          Hit(x: 0.0, y: 0.0, w: 88.0, h: 26.0, key: 14500),
                      ]),
                Stack(x: 0.0, y: 0.0, w: 88.0, h: 26.0, pad: 0.0,
                      items: [
                          Surface(x: 0.0, y: 0.0, w: 88.0, h: 26.0, r: 13.0, color: value("quote_save_fill")),
                          Hit(x: 0.0, y: 0.0, w: 88.0, h: 26.0, key: 14501, visible: "quote_has"),
                      ]),
            ]),
        Text(x: -50.0, y: 13.0, w: 0.0, center_x: true, bottom: true,
             text: "\u{f01e}  new", font_size: 10.0, font: ui, weight: regular,
             color: value("quote_new_ink")),
        Text(x: 50.0, y: 13.0, w: 0.0, center_x: true, bottom: true,
             text: "\u{f0c7}  save", font_size: 10.0, font: ui, weight: regular,
             color: value("quote_save_ink")),
    ],
)
]==]

function draw(ctx)
    ctx.ui.scene(SCENE)
end
