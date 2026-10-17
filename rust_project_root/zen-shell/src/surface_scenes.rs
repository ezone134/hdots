//! GENERATED (one-shot conversion pass) — do not hand-edit.
//!
//! One JSON const per RETIRED whole-shell surface: the exact SURFACE
//! table the Lua module once fed `ctx.ui.surface_decl`, extracted and
//! serialized verbatim. A module only got converted after it matched
//! the pure-decl shape (draw = exactly one `ctx.ui.surface_decl(SURFACE)`,
//! no api reads).
use std::sync::OnceLock;
use crate::scene::SurfaceScene;

const BT_MENU_SCENE_JSON: &str = r#"{
  "name": "bt_menu",
  "stores": [],
  "items": [
    {
      "Hit": {
        "x": 12.0,
        "y": 8.0,
        "w": 28.0,
        "h": 28.0,
        "anchors": null,
        "r": 0.0,
        "surface": null,
        "surface_hover": null,
        "hover_only": false,
        "text": "",
        "font_size": 0.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "center_x": false,
        "dy": 0.0,
        "label_dy": 0.0,
        "icon": false,
        "glyph": null,
        "glyph_pad": 8.0,
        "bottom": false,
        "glyph_color": null,
        "glyph_color_hover": null,
        "color": null,
        "color_hover": null,
        "region_dx": 0.0,
        "region_dy": 0.0,
        "region_dw": 0.0,
        "region_dh": 0.0,
        "key": 1.0,
        "key_add": 0,
        "action": null,
        "visible": null
      }
    },
    {
      "Text": {
        "x": 22.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "",
        "font_size": 14.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": true,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 48.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "Bluetooth",
        "font_size": 15.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 16.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{pill_bt_meta}",
        "font_size": "pill_bt_meta_size",
        "font": "ui",
        "weight": "light",
        "right": true,
        "center": false,
        "icon": false,
        "color": {
          "value": "pill_bt_meta_col"
        },
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Rows": {
        "name": "bt_menu_rows",
        "y": 56.0,
        "w": 0.0,
        "row_h": 36.0,
        "pitch": 42.0,
        "pad": 16.0,
        "row_dy": 10.0,
        "r": 12.0,
        "cols": [
          {
            "x": 14.0,
            "w": 0.0,
            "right": false,
            "center": false,
            "font": "ui",
            "size": 12.0,
            "dy": null,
            "weight": "regular",
            "color": "fg",
            "icon": false,
            "hide": false,
            "edge": null,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          },
          {
            "x": 0.0,
            "w": 0.0,
            "right": true,
            "center": false,
            "font": "ui",
            "size": 11.0,
            "dy": null,
            "weight": "regular",
            "color": null,
            "icon": false,
            "hide": false,
            "edge": 28.0,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          }
        ],
        "key_base": 10,
        "key_stride": 1,
        "region_dy": 0.0,
        "region_dh": 0.0,
        "aux": null,
        "hover_surface": null,
        "surface_dy": 0.0,
        "surface_dh": 0.0,
        "max_rows": 0,
        "start": null,
        "check": null,
        "check_x": 2.0,
        "del_base": 0,
        "del_pad": 20.0,
        "del_dy": 1.0,
        "del_size": 9.0,
        "del_rx": 24.0,
        "del_ry": 4.0,
        "del_rh": 22.0,
        "del_color": null,
        "visible": null,
        "hairline": false,
        "hover_bar": false,
        "flash": null,
        "karaoke": null,
        "karaoke_fs": 10.5,
        "bottom": 0.0,
        "more": null,
        "more_dy": 2.0,
        "more_fs": 8.0,
        "more_color": null
      }
    },
    {
      "Text": {
        "x": 24.0,
        "y": 56.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{bt_empty}",
        "font_size": 12.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg3",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "bt_empty_on",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    }
  ]
}"#;

const DASHBOARD_SCENE_JSON: &str = r#"{
  "name": "dashboard",
  "stores": [],
  "items": [
    {
      "Repeat": {
        "name": "",
        "model": "dash_cards",
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "pad": 0.0,
        "spacing": 0.0,
        "visible": null,
        "auto": false,
        "dir": "free",
        "slot_key": null,
        "templates": [
          {
            "tag": "*",
            "items": [
              {
                "Surface": {
                  "x": "item.x",
                  "y": "item.y",
                  "w": "item.w",
                  "h": "item.h",
                  "r": 0.0,
                  "r_var": "dash_card_r",
                  "visible": null,
                  "hover_key": null,
                  "hover_key_var": null,
                  "hover_key_add": 0,
                  "color": {
                    "value": "dash_card_bg"
                  },
                  "hover_color": null,
                  "right": false,
                  "center_x": false,
                  "center_y": false,
                  "bottom": false,
                  "tr": 0.0,
                  "br": 0.0,
                  "bl": 0.0,
                  "stroke": 0.0,
                  "stroke_color": null
                }
              },
              {
                "Card": {
                  "name": "{item.card}",
                  "x": "item.x",
                  "y": "item.y",
                  "w": "item.w",
                  "h": "item.h",
                  "key": "item.key",
                  "visible": null
                }
              }
            ]
          }
        ]
      }
    },
    {
      "Surface": {
        "x": 0.0,
        "y": 12.0,
        "w": 0.0,
        "h": 26.0,
        "r": 16.0,
        "r_var": null,
        "visible": "banner_shown",
        "hover_key": null,
        "hover_key_var": null,
        "hover_key_add": 0,
        "color": {
          "value": "dash_opaque"
        },
        "hover_color": null,
        "right": false,
        "center_x": false,
        "center_y": false,
        "bottom": false,
        "tr": -16.0,
        "br": 0.0,
        "bl": 0.0,
        "stroke": 0.0,
        "stroke_color": null
      }
    },
    {
      "BannerRow": {
        "cells": "banner_cells_v",
        "x": 0.0,
        "y": 12.0,
        "w": 0.0,
        "h": 26.0,
        "visible": "banner_shown",
        "overflow": "banner_overflow",
        "zones": "banner_zones"
      }
    },
    {
      "Stack": {
        "x": "clamp(conn_cx - 75.0, 8.0, max(parent.w - 158.0, 8.0))",
        "y": 46.0,
        "w": 150.0,
        "h": 66.0,
        "pad": 0.0,
        "visible": "conn_open",
        "auto": false,
        "items": [
          {
            "Surface": {
              "x": -3.0,
              "y": -3.0,
              "w": 156.0,
              "h": 72.0,
              "r": 12.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "conn_shadow"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Surface": {
              "x": 0.0,
              "y": 0.0,
              "w": 150.0,
              "h": 66.0,
              "r": 10.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "conn_body"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Stack": {
              "x": 0.0,
              "y": 3.0,
              "w": 150.0,
              "h": 30.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Hit": {
                    "x": 3.0,
                    "y": 0.0,
                    "w": 144.0,
                    "h": 30.0,
                    "anchors": null,
                    "r": 8.0,
                    "surface": "hover_hl",
                    "surface_hover": null,
                    "hover_only": true,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 3.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 34360.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                },
                {
                  "Text": {
                    "x": 12.0,
                    "y": 9.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 13.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": {
                      "value": "conn_wifi_ink"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 10.0,
                    "y": 10.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "Wi-Fi",
                    "font_size": 11.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": "fg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 41.9,
                    "y": 10.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "{conn_wifi_text}",
                    "font_size": 11.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": false,
                    "color": {
                      "value": "conn_wifi_ink"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 12.0,
                    "y": 10.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 11.0,
                    "font": "ui",
                    "weight": "light",
                    "right": true,
                    "center": false,
                    "icon": true,
                    "color": "sfg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 0.0,
              "y": 33.0,
              "w": 150.0,
              "h": 30.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Hit": {
                    "x": 3.0,
                    "y": 0.0,
                    "w": 144.0,
                    "h": 30.0,
                    "anchors": null,
                    "r": 8.0,
                    "surface": "hover_hl",
                    "surface_hover": null,
                    "hover_only": true,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 3.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 34361.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                },
                {
                  "Text": {
                    "x": 12.0,
                    "y": 9.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 13.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": {
                      "value": "conn_bt_ink"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 10.0,
                    "y": 10.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "Bluetooth",
                    "font_size": 11.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": "fg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 67.42,
                    "y": 10.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "{conn_bt_text}",
                    "font_size": 11.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": false,
                    "color": {
                      "value": "conn_bt_ink"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 12.0,
                    "y": 10.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 11.0,
                    "font": "ui",
                    "weight": "light",
                    "right": true,
                    "center": false,
                    "icon": true,
                    "color": "sfg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                }
              ]
            }
          }
        ]
      }
    }
  ]
}"#;

const LOCK_SCENE_JSON: &str = r#"{
  "name": "lock",
  "stores": [],
  "items": [
    {
      "Surface": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "r": 0.0,
        "r_var": null,
        "visible": null,
        "hover_key": null,
        "hover_key_var": null,
        "hover_key_add": 0,
        "color": {
          "value": "lock_dim"
        },
        "hover_color": null,
        "right": false,
        "center_x": false,
        "center_y": false,
        "bottom": false,
        "tr": 0.0,
        "br": 0.0,
        "bl": 0.0,
        "stroke": 0.0,
        "stroke_color": null
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": -324.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{clock}",
        "font_size": 64.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": true,
        "icon": false,
        "color": "fg",
        "center_x": true,
        "center_y": true,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": -250.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{lock_date}",
        "font_size": 16.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": true,
        "icon": false,
        "color": "fg3",
        "center_x": true,
        "center_y": true,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Surface": {
        "x": 0.0,
        "y": -206.0,
        "w": 52.0,
        "h": 52.0,
        "r": 26.0,
        "r_var": null,
        "visible": null,
        "hover_key": null,
        "hover_key_var": null,
        "hover_key_add": 0,
        "color": {
          "value": "lock_avatar"
        },
        "hover_color": null,
        "right": false,
        "center_x": true,
        "center_y": true,
        "bottom": false,
        "tr": 0.0,
        "br": 0.0,
        "bl": 0.0,
        "stroke": 0.0,
        "stroke_color": null
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": -192.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{user_init}",
        "font_size": 22.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": true,
        "icon": false,
        "color": "fg",
        "center_x": true,
        "center_y": true,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": -150.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{username}",
        "font_size": 15.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": true,
        "icon": false,
        "color": "fg2",
        "center_x": true,
        "center_y": true,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Stack": {
        "x": "parent.w / 2 - 170.0",
        "y": "max(parent.h * 0.45, parent.h * 0.2 + 230.0)",
        "w": 340.0,
        "h": 58.0,
        "pad": 0.0,
        "visible": null,
        "auto": false,
        "items": [
          {
            "Surface": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "r": 16.0,
              "r_var": null,
              "visible": null,
              "hover_key": 1.0,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "hover",
              "hover_color": "hover_hl",
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 0.0,
              "y": 22.0,
              "w": 0.0,
              "h": 0.0,
              "text": "{lock_pw_text}",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": true,
              "icon": false,
              "color": {
                "value": "lock_pw_ink"
              },
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Hit": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 1.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          }
        ]
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": "max(parent.h * 0.45, parent.h * 0.2 + 230.0) + 80.0",
        "w": 0.0,
        "h": 0.0,
        "text": "{lock_err}",
        "font_size": 13.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": true,
        "icon": false,
        "color": "danger",
        "center_x": true,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "lock_err_on",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": "max(parent.h * 0.45, parent.h * 0.2 + 230.0) + 80.0 + lock_err_shown * 22.0",
        "w": 0.0,
        "h": 0.0,
        "text": "{lock_caps}",
        "font_size": 13.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": true,
        "icon": true,
        "color": "danger",
        "center_x": true,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "lock_caps_on",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Stack": {
        "x": "parent.w / 2 - 241.0",
        "y": "parent.h - 120.0",
        "w": 150.0,
        "h": 52.0,
        "pad": 0.0,
        "visible": null,
        "auto": false,
        "items": [
          {
            "Surface": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "r": 14.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "lock_bg_0"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Bar": {
              "x": 2.0,
              "y": -2.0,
              "w": 146.0,
              "h": 52.0,
              "r": 13.0,
              "max": 1.0,
              "value": "{lock_hold_0}",
              "fill": {
                "value": "lock_hold_fill"
              },
              "track": {
                "value": "clear"
              },
              "reserve": 0.0,
              "vertical": true,
              "visible": "lock_hold_on_0"
            }
          },
          {
            "Text": {
              "x": 66.0,
              "y": 10.0,
              "w": 0.0,
              "h": 0.0,
              "text": "",
              "font_size": 18.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 0.0,
              "y": 32.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Logout",
              "font_size": 12.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": true,
              "icon": false,
              "color": "fg2",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Hit": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 2.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          }
        ]
      }
    },
    {
      "Stack": {
        "x": "parent.w / 2 - 75.0",
        "y": "parent.h - 120.0",
        "w": 150.0,
        "h": 52.0,
        "pad": 0.0,
        "visible": null,
        "auto": false,
        "items": [
          {
            "Surface": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "r": 14.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "lock_bg_1"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Bar": {
              "x": 2.0,
              "y": -2.0,
              "w": 146.0,
              "h": 52.0,
              "r": 13.0,
              "max": 1.0,
              "value": "{lock_hold_1}",
              "fill": {
                "value": "lock_hold_fill"
              },
              "track": {
                "value": "clear"
              },
              "reserve": 0.0,
              "vertical": true,
              "visible": "lock_hold_on_1"
            }
          },
          {
            "Text": {
              "x": 66.0,
              "y": 10.0,
              "w": 0.0,
              "h": 0.0,
              "text": "",
              "font_size": 18.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 0.0,
              "y": 32.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Restart",
              "font_size": 12.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": true,
              "icon": false,
              "color": "fg2",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Hit": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 3.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          }
        ]
      }
    },
    {
      "Stack": {
        "x": "parent.w / 2 + 91.0",
        "y": "parent.h - 120.0",
        "w": 150.0,
        "h": 52.0,
        "pad": 0.0,
        "visible": null,
        "auto": false,
        "items": [
          {
            "Surface": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "r": 14.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "lock_bg_2"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Bar": {
              "x": 2.0,
              "y": -2.0,
              "w": 146.0,
              "h": 52.0,
              "r": 13.0,
              "max": 1.0,
              "value": "{lock_hold_2}",
              "fill": {
                "value": "lock_hold_fill"
              },
              "track": {
                "value": "clear"
              },
              "reserve": 0.0,
              "vertical": true,
              "visible": "lock_hold_on_2"
            }
          },
          {
            "Text": {
              "x": 66.0,
              "y": 10.0,
              "w": 0.0,
              "h": 0.0,
              "text": "",
              "font_size": 18.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "danger",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 0.0,
              "y": 32.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Shutdown",
              "font_size": 12.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": true,
              "icon": false,
              "color": "fg2",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Hit": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 4.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          }
        ]
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": "parent.h - 46.0",
        "w": 0.0,
        "h": 0.0,
        "text": "{lock_hint}",
        "font_size": 10.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": true,
        "icon": false,
        "color": "fg3",
        "center_x": true,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    }
  ]
}"#;

const NOTIF_SCENE_JSON: &str = r#"{
  "name": "notif",
  "stores": [],
  "items": [
    {
      "Text": {
        "x": 16.0,
        "y": 12.0,
        "w": 0.0,
        "h": 0.0,
        "text": "Notifications",
        "font_size": 15.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": null,
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Hit": {
        "x": 112.0,
        "y": 10.0,
        "w": 84.0,
        "h": 30.0,
        "anchors": null,
        "r": 15.0,
        "surface": "raised",
        "surface_hover": "raised_hl",
        "hover_only": false,
        "text": "",
        "font_size": 0.0,
        "font": "ui",
        "weight": "light",
        "right": true,
        "center": false,
        "center_x": false,
        "dy": 0.0,
        "label_dy": 0.0,
        "icon": false,
        "glyph": null,
        "glyph_pad": 8.0,
        "bottom": false,
        "glyph_color": null,
        "glyph_color_hover": null,
        "color": null,
        "color_hover": null,
        "region_dx": 0.0,
        "region_dy": 0.0,
        "region_dw": 0.0,
        "region_dh": 0.0,
        "key": 2.0,
        "key_add": 0,
        "action": null,
        "visible": null
      }
    },
    {
      "Text": {
        "x": 230.0,
        "y": 17.0,
        "w": 0.0,
        "h": 0.0,
        "text": "Clear",
        "font_size": 11.0,
        "font": "ui",
        "weight": "semibold",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Hit": {
        "x": 16.0,
        "y": 10.0,
        "w": 88.0,
        "h": 30.0,
        "anchors": null,
        "r": 15.0,
        "surface": {
          "value": "dnd_chip"
        },
        "surface_hover": {
          "value": "dnd_chip_hover"
        },
        "hover_only": false,
        "text": "",
        "font_size": 0.0,
        "font": "ui",
        "weight": "light",
        "right": true,
        "center": false,
        "center_x": false,
        "dy": 0.0,
        "label_dy": 0.0,
        "icon": false,
        "glyph": null,
        "glyph_pad": 8.0,
        "bottom": false,
        "glyph_color": null,
        "glyph_color_hover": null,
        "color": null,
        "color_hover": null,
        "region_dx": 0.0,
        "region_dy": 0.0,
        "region_dw": 0.0,
        "region_dh": 0.0,
        "key": 3.0,
        "key_add": 0,
        "action": null,
        "visible": null
      }
    },
    {
      "Text": {
        "x": 324.0,
        "y": 17.0,
        "w": 0.0,
        "h": 0.0,
        "text": " DND",
        "font_size": 11.0,
        "font": "ui",
        "weight": "semibold",
        "right": false,
        "center": false,
        "icon": false,
        "color": {
          "value": "dnd_chip_fg"
        },
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Repeat": {
        "name": "",
        "model": "notif_rows",
        "x": 16.0,
        "y": 52.0,
        "w": -16.0,
        "h": 46.0,
        "pad": 0.0,
        "spacing": 6.0,
        "visible": null,
        "auto": false,
        "dir": "down",
        "slot_key": null,
        "templates": [
          {
            "tag": "card",
            "items": [
              {
                "Stack": {
                  "x": 0.0,
                  "y": 0.0,
                  "w": 0.0,
                  "h": 0.0,
                  "pad": 0.0,
                  "visible": null,
                  "auto": false,
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 0.0,
                        "r": 12.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": "item.key",
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "raised",
                        "hover_color": "raised_hl",
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": 4.0,
                        "y": 10.0,
                        "w": 4.0,
                        "h": 26.0,
                        "r": 2.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.strip"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Image": {
                        "x": 16.0,
                        "y": 4.0,
                        "w": 20.0,
                        "h": 20.0,
                        "key": "{item.icon}",
                        "fallback": null,
                        "right": false,
                        "visible": "item.has_icon"
                      }
                    },
                    {
                      "Text": {
                        "x": 44.0,
                        "y": 6.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.app}",
                        "font_size": 10.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": "fg3",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 44.0,
                        "y": 18.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.summary}",
                        "font_size": 13.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": "fg",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 44.0,
                        "y": 32.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.body}",
                        "font_size": 11.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": "fg2",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": "item.has_body",
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 0.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              }
            ]
          }
        ]
      }
    },
    {
      "Text": {
        "x": 24.0,
        "y": 70.0,
        "w": 0.0,
        "h": 0.0,
        "text": "No notifications",
        "font_size": 12.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg3",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "notif_none",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    }
  ]
}"#;

const OSD_SCENE_JSON: &str = r#"{
  "name": "osd",
  "stores": [],
  "items": [
    {
      "Hit": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "anchors": null,
        "r": 0.0,
        "surface": null,
        "surface_hover": null,
        "hover_only": false,
        "text": "",
        "font_size": 0.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "center_x": false,
        "dy": 0.0,
        "label_dy": 0.0,
        "icon": false,
        "glyph": null,
        "glyph_pad": 8.0,
        "bottom": false,
        "glyph_color": null,
        "glyph_color_hover": null,
        "color": null,
        "color_hover": null,
        "region_dx": 0.0,
        "region_dy": 0.0,
        "region_dw": 0.0,
        "region_dh": 0.0,
        "key": 1.0,
        "key_add": 0,
        "action": null,
        "visible": null
      }
    },
    {
      "Text": {
        "x": 18.0,
        "y": -11.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{osd_glyph}",
        "font_size": 22.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": true,
        "color": "acc",
        "center_x": false,
        "center_y": true,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "osd_on",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 52.0,
        "y": -7.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{osd_text}",
        "font_size": 14.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": false,
        "center_y": true,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "osd_on",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    }
  ]
}"#;

const PILL_SCENE_JSON: &str = r#"{
  "name": "pill",
  "stores": [],
  "items": [
    {
      "Row": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "pad": 0.0,
        "spacing": 0.0,
        "valign": "top",
        "halign": "left",
        "bottom": false,
        "min_card_h": 0.0,
        "visible": null,
        "auto": false,
        "items": [
          {
            "Row": {
              "x": "pill_mx",
              "y": 0.0,
              "w": "pill_left_w",
              "h": 0.0,
              "pad": 0.0,
              "spacing": 6.0,
              "valign": "top",
              "halign": "left",
              "bottom": false,
              "min_card_h": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Stack": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": "pill_clock_w",
                    "h": 0.0,
                    "pad": 0.0,
                    "visible": "pill_clock_on",
                    "auto": false,
                    "items": [
                      {
                        "Text": {
                          "x": 0.0,
                          "y": "pill_mid",
                          "w": 0.0,
                          "h": 0.0,
                          "text": "{pill_clock}",
                          "font_size": 14.0,
                          "font": "ui",
                          "weight": "light",
                          "right": false,
                          "center": false,
                          "icon": false,
                          "color": {
                            "value": "pill_clock_ink"
                          },
                          "center_x": false,
                          "center_y": false,
                          "cy_pct": 0.0,
                          "cx_pct": 0.0,
                          "bottom": false,
                          "visible": null,
                          "size_ladder": [],
                          "y_ladder": [],
                          "dy": 0.0,
                          "auto": false
                        }
                      },
                      {
                        "Hit": {
                          "x": 0.0,
                          "y": 0.0,
                          "w": 0.0,
                          "h": 0.0,
                          "anchors": {
                            "left": -2.0,
                            "top": 4.0,
                            "right": 2.0,
                            "bottom": -4.0
                          },
                          "r": 0.0,
                          "surface": null,
                          "surface_hover": null,
                          "hover_only": false,
                          "text": "",
                          "font_size": 0.0,
                          "font": "ui",
                          "weight": "light",
                          "right": false,
                          "center": false,
                          "center_x": false,
                          "dy": 0.0,
                          "label_dy": 0.0,
                          "icon": false,
                          "glyph": null,
                          "glyph_pad": 8.0,
                          "bottom": false,
                          "glyph_color": null,
                          "glyph_color_hover": null,
                          "color": null,
                          "color_hover": null,
                          "region_dx": 0.0,
                          "region_dy": 0.0,
                          "region_dw": 0.0,
                          "region_dh": 0.0,
                          "key": "pill_clock_key",
                          "key_add": 0,
                          "action": null,
                          "visible": null
                        }
                      }
                    ]
                  }
                },
                {
                  "Stack": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": "pill_date_w",
                    "h": 0.0,
                    "pad": 0.0,
                    "visible": null,
                    "auto": false,
                    "items": [
                      {
                        "Text": {
                          "x": 0.0,
                          "y": "pill_mid",
                          "w": 0.0,
                          "h": 0.0,
                          "text": "{pill_date}",
                          "font_size": 14.0,
                          "font": "ui",
                          "weight": "light",
                          "right": false,
                          "center": false,
                          "icon": false,
                          "color": {
                            "value": "pill_date_ink"
                          },
                          "center_x": false,
                          "center_y": false,
                          "cy_pct": 0.0,
                          "cx_pct": 0.0,
                          "bottom": false,
                          "visible": null,
                          "size_ladder": [],
                          "y_ladder": [],
                          "dy": 0.0,
                          "auto": false
                        }
                      },
                      {
                        "Hit": {
                          "x": 0.0,
                          "y": 0.0,
                          "w": 0.0,
                          "h": 0.0,
                          "anchors": {
                            "left": -2.0,
                            "top": 4.0,
                            "right": 2.0,
                            "bottom": -4.0
                          },
                          "r": 0.0,
                          "surface": null,
                          "surface_hover": null,
                          "hover_only": false,
                          "text": "",
                          "font_size": 0.0,
                          "font": "ui",
                          "weight": "light",
                          "right": false,
                          "center": false,
                          "center_x": false,
                          "dy": 0.0,
                          "label_dy": 0.0,
                          "icon": false,
                          "glyph": null,
                          "glyph_pad": 8.0,
                          "bottom": false,
                          "glyph_color": null,
                          "glyph_color_hover": null,
                          "color": null,
                          "color_hover": null,
                          "region_dx": 0.0,
                          "region_dy": 0.0,
                          "region_dw": 0.0,
                          "region_dh": 0.0,
                          "key": "pill_date_key",
                          "key_add": 0,
                          "action": null,
                          "visible": null
                        }
                      }
                    ]
                  }
                },
                {
                  "Stack": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": "pill_batt_w",
                    "h": 0.0,
                    "pad": 0.0,
                    "visible": "pill_batt_on",
                    "auto": false,
                    "items": [
                      {
                        "Text": {
                          "x": 0.0,
                          "y": "pill_mid_sm",
                          "w": 0.0,
                          "h": 0.0,
                          "text": "{pill_batt_glyph}",
                          "font_size": 13.0,
                          "font": "ui",
                          "weight": "light",
                          "right": false,
                          "center": false,
                          "icon": true,
                          "color": {
                            "value": "pill_batt_ink"
                          },
                          "center_x": false,
                          "center_y": false,
                          "cy_pct": 0.0,
                          "cx_pct": 0.0,
                          "bottom": false,
                          "visible": null,
                          "size_ladder": [],
                          "y_ladder": [],
                          "dy": 0.0,
                          "auto": false
                        }
                      },
                      {
                        "Text": {
                          "x": "pill_batt_pct_x",
                          "y": "pill_mid",
                          "w": 0.0,
                          "h": 0.0,
                          "text": "{pill_batt_pct}",
                          "font_size": 14.0,
                          "font": "ui",
                          "weight": "light",
                          "right": false,
                          "center": false,
                          "icon": false,
                          "color": {
                            "value": "pill_batt_ink"
                          },
                          "center_x": false,
                          "center_y": false,
                          "cy_pct": 0.0,
                          "cx_pct": 0.0,
                          "bottom": false,
                          "visible": "pill_batt_pct_on",
                          "size_ladder": [],
                          "y_ladder": [],
                          "dy": 0.0,
                          "auto": false
                        }
                      },
                      {
                        "Hit": {
                          "x": 0.0,
                          "y": 0.0,
                          "w": 0.0,
                          "h": 0.0,
                          "anchors": {
                            "left": -2.0,
                            "top": 4.0,
                            "right": 2.0,
                            "bottom": -4.0
                          },
                          "r": 0.0,
                          "surface": null,
                          "surface_hover": null,
                          "hover_only": false,
                          "text": "",
                          "font_size": 0.0,
                          "font": "ui",
                          "weight": "light",
                          "right": false,
                          "center": false,
                          "center_x": false,
                          "dy": 0.0,
                          "label_dy": 0.0,
                          "icon": false,
                          "glyph": null,
                          "glyph_pad": 8.0,
                          "bottom": false,
                          "glyph_color": null,
                          "glyph_color_hover": null,
                          "color": null,
                          "color_hover": null,
                          "region_dx": 0.0,
                          "region_dy": 0.0,
                          "region_dw": 0.0,
                          "region_dh": 0.0,
                          "key": "pill_batt_key",
                          "key_add": 0,
                          "action": null,
                          "visible": null
                        }
                      }
                    ]
                  }
                }
              ]
            }
          },
          {
            "Row": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "pad": 0.0,
              "spacing": 0.0,
              "valign": "top",
              "halign": "center",
              "bottom": false,
              "min_card_h": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Repeat": {
                    "name": "",
                    "model": "pill_ctr",
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "pad": 0.0,
                    "spacing": 6.0,
                    "visible": null,
                    "auto": true,
                    "dir": "across",
                    "slot_key": null,
                    "templates": [
                      {
                        "tag": "ws_number",
                        "items": [
                          {
                            "Stack": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": "item.w",
                              "h": 0.0,
                              "pad": 0.0,
                              "visible": null,
                              "auto": false,
                              "items": [
                                {
                                  "Text": {
                                    "x": 2.0,
                                    "y": "pill_mid",
                                    "w": 0.0,
                                    "h": 0.0,
                                    "text": "{pill_ws_n}",
                                    "font_size": 14.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "icon": false,
                                    "color": {
                                      "value": "item.ink"
                                    },
                                    "center_x": false,
                                    "center_y": false,
                                    "cy_pct": 0.0,
                                    "cx_pct": 0.0,
                                    "bottom": false,
                                    "visible": null,
                                    "size_ladder": [],
                                    "y_ladder": [],
                                    "dy": 0.0,
                                    "auto": false
                                  }
                                },
                                {
                                  "Hit": {
                                    "x": -4.0,
                                    "y": 8.0,
                                    "w": "pill_ws_reg_w",
                                    "h": -4.0,
                                    "anchors": null,
                                    "r": 0.0,
                                    "surface": null,
                                    "surface_hover": null,
                                    "hover_only": false,
                                    "text": "",
                                    "font_size": 0.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "center_x": false,
                                    "dy": 0.0,
                                    "label_dy": 0.0,
                                    "icon": false,
                                    "glyph": null,
                                    "glyph_pad": 8.0,
                                    "bottom": false,
                                    "glyph_color": null,
                                    "glyph_color_hover": null,
                                    "color": null,
                                    "color_hover": null,
                                    "region_dx": 0.0,
                                    "region_dy": 0.0,
                                    "region_dw": 0.0,
                                    "region_dh": 0.0,
                                    "key": "item.key",
                                    "key_add": 0,
                                    "action": null,
                                    "visible": null
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      },
                      {
                        "tag": "ws_long",
                        "items": [
                          {
                            "Row": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": "pill_ws_long_w",
                              "h": 0.0,
                              "pad": 0.0,
                              "spacing": 2.0,
                              "valign": "top",
                              "halign": "left",
                              "bottom": false,
                              "min_card_h": 0.0,
                              "visible": null,
                              "auto": false,
                              "items": [
                                {
                                  "Stack": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 18.0,
                                    "h": 0.0,
                                    "pad": 0.0,
                                    "visible": null,
                                    "auto": false,
                                    "items": [
                                      {
                                        "Hit": {
                                          "x": 0.0,
                                          "y": 7.0,
                                          "w": 18.0,
                                          "h": 24.0,
                                          "anchors": null,
                                          "r": 6.0,
                                          "surface": "hover_hl",
                                          "surface_hover": null,
                                          "hover_only": true,
                                          "text": "",
                                          "font_size": 0.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "center_x": false,
                                          "dy": 0.0,
                                          "label_dy": 0.0,
                                          "icon": false,
                                          "glyph": null,
                                          "glyph_pad": 8.0,
                                          "bottom": false,
                                          "glyph_color": null,
                                          "glyph_color_hover": null,
                                          "color": null,
                                          "color_hover": null,
                                          "region_dx": 0.0,
                                          "region_dy": 0.0,
                                          "region_dw": 0.0,
                                          "region_dh": 0.0,
                                          "key": "item.key",
                                          "key_add": 0,
                                          "action": null,
                                          "visible": null
                                        }
                                      },
                                      {
                                        "Text": {
                                          "x": 6.0,
                                          "y": "pill_mid",
                                          "w": 0.0,
                                          "h": 0.0,
                                          "text": "1",
                                          "font_size": 14.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "icon": false,
                                          "color": "fg",
                                          "center_x": false,
                                          "center_y": false,
                                          "cy_pct": 0.0,
                                          "cx_pct": 0.0,
                                          "bottom": false,
                                          "visible": null,
                                          "size_ladder": [],
                                          "y_ladder": [],
                                          "dy": 0.0,
                                          "auto": false
                                        }
                                      },
                                      {
                                        "Surface": {
                                          "x": 7.5,
                                          "y": 32.0,
                                          "w": 3.0,
                                          "h": 3.0,
                                          "r": 1.5,
                                          "r_var": null,
                                          "visible": "pill_ws_long_on_1",
                                          "hover_key": null,
                                          "hover_key_var": null,
                                          "hover_key_add": 0,
                                          "color": "acc",
                                          "hover_color": null,
                                          "right": false,
                                          "center_x": false,
                                          "center_y": false,
                                          "bottom": false,
                                          "tr": 0.0,
                                          "br": 0.0,
                                          "bl": 0.0,
                                          "stroke": 0.0,
                                          "stroke_color": null
                                        }
                                      }
                                    ]
                                  }
                                },
                                {
                                  "Stack": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 18.0,
                                    "h": 0.0,
                                    "pad": 0.0,
                                    "visible": null,
                                    "auto": false,
                                    "items": [
                                      {
                                        "Hit": {
                                          "x": 0.0,
                                          "y": 7.0,
                                          "w": 18.0,
                                          "h": 24.0,
                                          "anchors": null,
                                          "r": 6.0,
                                          "surface": "hover_hl",
                                          "surface_hover": null,
                                          "hover_only": true,
                                          "text": "",
                                          "font_size": 0.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "center_x": false,
                                          "dy": 0.0,
                                          "label_dy": 0.0,
                                          "icon": false,
                                          "glyph": null,
                                          "glyph_pad": 8.0,
                                          "bottom": false,
                                          "glyph_color": null,
                                          "glyph_color_hover": null,
                                          "color": null,
                                          "color_hover": null,
                                          "region_dx": 0.0,
                                          "region_dy": 0.0,
                                          "region_dw": 0.0,
                                          "region_dh": 0.0,
                                          "key": "item.key",
                                          "key_add": 1,
                                          "action": null,
                                          "visible": null
                                        }
                                      },
                                      {
                                        "Text": {
                                          "x": 6.0,
                                          "y": "pill_mid",
                                          "w": 0.0,
                                          "h": 0.0,
                                          "text": "2",
                                          "font_size": 14.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "icon": false,
                                          "color": "fg",
                                          "center_x": false,
                                          "center_y": false,
                                          "cy_pct": 0.0,
                                          "cx_pct": 0.0,
                                          "bottom": false,
                                          "visible": null,
                                          "size_ladder": [],
                                          "y_ladder": [],
                                          "dy": 0.0,
                                          "auto": false
                                        }
                                      },
                                      {
                                        "Surface": {
                                          "x": 7.5,
                                          "y": 32.0,
                                          "w": 3.0,
                                          "h": 3.0,
                                          "r": 1.5,
                                          "r_var": null,
                                          "visible": "pill_ws_long_on_2",
                                          "hover_key": null,
                                          "hover_key_var": null,
                                          "hover_key_add": 0,
                                          "color": "acc",
                                          "hover_color": null,
                                          "right": false,
                                          "center_x": false,
                                          "center_y": false,
                                          "bottom": false,
                                          "tr": 0.0,
                                          "br": 0.0,
                                          "bl": 0.0,
                                          "stroke": 0.0,
                                          "stroke_color": null
                                        }
                                      }
                                    ]
                                  }
                                },
                                {
                                  "Stack": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 18.0,
                                    "h": 0.0,
                                    "pad": 0.0,
                                    "visible": null,
                                    "auto": false,
                                    "items": [
                                      {
                                        "Hit": {
                                          "x": 0.0,
                                          "y": 7.0,
                                          "w": 18.0,
                                          "h": 24.0,
                                          "anchors": null,
                                          "r": 6.0,
                                          "surface": "hover_hl",
                                          "surface_hover": null,
                                          "hover_only": true,
                                          "text": "",
                                          "font_size": 0.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "center_x": false,
                                          "dy": 0.0,
                                          "label_dy": 0.0,
                                          "icon": false,
                                          "glyph": null,
                                          "glyph_pad": 8.0,
                                          "bottom": false,
                                          "glyph_color": null,
                                          "glyph_color_hover": null,
                                          "color": null,
                                          "color_hover": null,
                                          "region_dx": 0.0,
                                          "region_dy": 0.0,
                                          "region_dw": 0.0,
                                          "region_dh": 0.0,
                                          "key": "item.key",
                                          "key_add": 2,
                                          "action": null,
                                          "visible": null
                                        }
                                      },
                                      {
                                        "Text": {
                                          "x": 6.0,
                                          "y": "pill_mid",
                                          "w": 0.0,
                                          "h": 0.0,
                                          "text": "3",
                                          "font_size": 14.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "icon": false,
                                          "color": "fg",
                                          "center_x": false,
                                          "center_y": false,
                                          "cy_pct": 0.0,
                                          "cx_pct": 0.0,
                                          "bottom": false,
                                          "visible": null,
                                          "size_ladder": [],
                                          "y_ladder": [],
                                          "dy": 0.0,
                                          "auto": false
                                        }
                                      },
                                      {
                                        "Surface": {
                                          "x": 7.5,
                                          "y": 32.0,
                                          "w": 3.0,
                                          "h": 3.0,
                                          "r": 1.5,
                                          "r_var": null,
                                          "visible": "pill_ws_long_on_3",
                                          "hover_key": null,
                                          "hover_key_var": null,
                                          "hover_key_add": 0,
                                          "color": "acc",
                                          "hover_color": null,
                                          "right": false,
                                          "center_x": false,
                                          "center_y": false,
                                          "bottom": false,
                                          "tr": 0.0,
                                          "br": 0.0,
                                          "bl": 0.0,
                                          "stroke": 0.0,
                                          "stroke_color": null
                                        }
                                      }
                                    ]
                                  }
                                },
                                {
                                  "Stack": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 18.0,
                                    "h": 0.0,
                                    "pad": 0.0,
                                    "visible": null,
                                    "auto": false,
                                    "items": [
                                      {
                                        "Hit": {
                                          "x": 0.0,
                                          "y": 7.0,
                                          "w": 18.0,
                                          "h": 24.0,
                                          "anchors": null,
                                          "r": 6.0,
                                          "surface": "hover_hl",
                                          "surface_hover": null,
                                          "hover_only": true,
                                          "text": "",
                                          "font_size": 0.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "center_x": false,
                                          "dy": 0.0,
                                          "label_dy": 0.0,
                                          "icon": false,
                                          "glyph": null,
                                          "glyph_pad": 8.0,
                                          "bottom": false,
                                          "glyph_color": null,
                                          "glyph_color_hover": null,
                                          "color": null,
                                          "color_hover": null,
                                          "region_dx": 0.0,
                                          "region_dy": 0.0,
                                          "region_dw": 0.0,
                                          "region_dh": 0.0,
                                          "key": "item.key",
                                          "key_add": 3,
                                          "action": null,
                                          "visible": null
                                        }
                                      },
                                      {
                                        "Text": {
                                          "x": 6.0,
                                          "y": "pill_mid",
                                          "w": 0.0,
                                          "h": 0.0,
                                          "text": "4",
                                          "font_size": 14.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "icon": false,
                                          "color": "fg",
                                          "center_x": false,
                                          "center_y": false,
                                          "cy_pct": 0.0,
                                          "cx_pct": 0.0,
                                          "bottom": false,
                                          "visible": null,
                                          "size_ladder": [],
                                          "y_ladder": [],
                                          "dy": 0.0,
                                          "auto": false
                                        }
                                      },
                                      {
                                        "Surface": {
                                          "x": 7.5,
                                          "y": 32.0,
                                          "w": 3.0,
                                          "h": 3.0,
                                          "r": 1.5,
                                          "r_var": null,
                                          "visible": "pill_ws_long_on_4",
                                          "hover_key": null,
                                          "hover_key_var": null,
                                          "hover_key_add": 0,
                                          "color": "acc",
                                          "hover_color": null,
                                          "right": false,
                                          "center_x": false,
                                          "center_y": false,
                                          "bottom": false,
                                          "tr": 0.0,
                                          "br": 0.0,
                                          "bl": 0.0,
                                          "stroke": 0.0,
                                          "stroke_color": null
                                        }
                                      }
                                    ]
                                  }
                                },
                                {
                                  "Stack": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 18.0,
                                    "h": 0.0,
                                    "pad": 0.0,
                                    "visible": null,
                                    "auto": false,
                                    "items": [
                                      {
                                        "Hit": {
                                          "x": 0.0,
                                          "y": 7.0,
                                          "w": 18.0,
                                          "h": 24.0,
                                          "anchors": null,
                                          "r": 6.0,
                                          "surface": "hover_hl",
                                          "surface_hover": null,
                                          "hover_only": true,
                                          "text": "",
                                          "font_size": 0.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "center_x": false,
                                          "dy": 0.0,
                                          "label_dy": 0.0,
                                          "icon": false,
                                          "glyph": null,
                                          "glyph_pad": 8.0,
                                          "bottom": false,
                                          "glyph_color": null,
                                          "glyph_color_hover": null,
                                          "color": null,
                                          "color_hover": null,
                                          "region_dx": 0.0,
                                          "region_dy": 0.0,
                                          "region_dw": 0.0,
                                          "region_dh": 0.0,
                                          "key": "item.key",
                                          "key_add": 4,
                                          "action": null,
                                          "visible": null
                                        }
                                      },
                                      {
                                        "Text": {
                                          "x": 6.0,
                                          "y": "pill_mid",
                                          "w": 0.0,
                                          "h": 0.0,
                                          "text": "5",
                                          "font_size": 14.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "icon": false,
                                          "color": "fg",
                                          "center_x": false,
                                          "center_y": false,
                                          "cy_pct": 0.0,
                                          "cx_pct": 0.0,
                                          "bottom": false,
                                          "visible": null,
                                          "size_ladder": [],
                                          "y_ladder": [],
                                          "dy": 0.0,
                                          "auto": false
                                        }
                                      },
                                      {
                                        "Surface": {
                                          "x": 7.5,
                                          "y": 32.0,
                                          "w": 3.0,
                                          "h": 3.0,
                                          "r": 1.5,
                                          "r_var": null,
                                          "visible": "pill_ws_long_on_5",
                                          "hover_key": null,
                                          "hover_key_var": null,
                                          "hover_key_add": 0,
                                          "color": "acc",
                                          "hover_color": null,
                                          "right": false,
                                          "center_x": false,
                                          "center_y": false,
                                          "bottom": false,
                                          "tr": 0.0,
                                          "br": 0.0,
                                          "bl": 0.0,
                                          "stroke": 0.0,
                                          "stroke_color": null
                                        }
                                      }
                                    ]
                                  }
                                },
                                {
                                  "Stack": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": "pill_ws_w",
                                    "h": 0.0,
                                    "pad": 0.0,
                                    "visible": "pill_ws_tail_on",
                                    "auto": false,
                                    "items": [
                                      {
                                        "Surface": {
                                          "x": 0.0,
                                          "y": 7.0,
                                          "w": "pill_ws_tail_hover_w",
                                          "h": 24.0,
                                          "r": 6.0,
                                          "r_var": null,
                                          "visible": null,
                                          "hover_key": "item.key",
                                          "hover_key_var": null,
                                          "hover_key_add": 5,
                                          "color": "hover_hl",
                                          "hover_color": null,
                                          "right": false,
                                          "center_x": false,
                                          "center_y": false,
                                          "bottom": false,
                                          "tr": 0.0,
                                          "br": 0.0,
                                          "bl": 0.0,
                                          "stroke": 0.0,
                                          "stroke_color": null
                                        }
                                      },
                                      {
                                        "Hit": {
                                          "x": -2.0,
                                          "y": 8.0,
                                          "w": "pill_ws_tail_reg_w",
                                          "h": -4.0,
                                          "anchors": null,
                                          "r": 0.0,
                                          "surface": null,
                                          "surface_hover": null,
                                          "hover_only": false,
                                          "text": "",
                                          "font_size": 0.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "center_x": false,
                                          "dy": 0.0,
                                          "label_dy": 0.0,
                                          "icon": false,
                                          "glyph": null,
                                          "glyph_pad": 8.0,
                                          "bottom": false,
                                          "glyph_color": null,
                                          "glyph_color_hover": null,
                                          "color": null,
                                          "color_hover": null,
                                          "region_dx": 0.0,
                                          "region_dy": 0.0,
                                          "region_dw": 0.0,
                                          "region_dh": 0.0,
                                          "key": "item.key",
                                          "key_add": 5,
                                          "action": null,
                                          "visible": null
                                        }
                                      },
                                      {
                                        "Text": {
                                          "x": 4.0,
                                          "y": "pill_mid",
                                          "w": 0.0,
                                          "h": 0.0,
                                          "text": "{pill_ws_tail_n}",
                                          "font_size": 14.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "icon": false,
                                          "color": "fg",
                                          "center_x": false,
                                          "center_y": false,
                                          "cy_pct": 0.0,
                                          "cx_pct": 0.0,
                                          "bottom": false,
                                          "visible": null,
                                          "size_ladder": [],
                                          "y_ladder": [],
                                          "dy": 0.0,
                                          "auto": false
                                        }
                                      },
                                      {
                                        "Surface": {
                                          "x": "pill_ws_tail_dot_x",
                                          "y": "pill_ws_tail_dot_y",
                                          "w": 3.0,
                                          "h": 3.0,
                                          "r": 1.5,
                                          "r_var": null,
                                          "visible": null,
                                          "hover_key": null,
                                          "hover_key_var": null,
                                          "hover_key_add": 0,
                                          "color": "acc",
                                          "hover_color": null,
                                          "right": false,
                                          "center_x": false,
                                          "center_y": false,
                                          "bottom": false,
                                          "tr": 0.0,
                                          "br": 0.0,
                                          "bl": 0.0,
                                          "stroke": 0.0,
                                          "stroke_color": null
                                        }
                                      }
                                    ]
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      },
                      {
                        "tag": "icon_chip",
                        "items": [
                          {
                            "Stack": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": "item.w",
                              "h": 0.0,
                              "pad": 0.0,
                              "visible": null,
                              "auto": false,
                              "items": [
                                {
                                  "Text": {
                                    "x": 4.0,
                                    "y": "pill_mid",
                                    "w": 0.0,
                                    "h": 0.0,
                                    "text": "{item.glyph}",
                                    "font_size": 14.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "icon": true,
                                    "color": {
                                      "value": "item.ink"
                                    },
                                    "center_x": false,
                                    "center_y": false,
                                    "cy_pct": 0.0,
                                    "cx_pct": 0.0,
                                    "bottom": false,
                                    "visible": null,
                                    "size_ladder": [],
                                    "y_ladder": [],
                                    "dy": 0.0,
                                    "auto": false
                                  }
                                },
                                {
                                  "Hit": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 0.0,
                                    "h": 0.0,
                                    "anchors": {
                                      "left": -2.0,
                                      "top": 4.0,
                                      "right": 2.0,
                                      "bottom": -4.0
                                    },
                                    "r": 0.0,
                                    "surface": null,
                                    "surface_hover": null,
                                    "hover_only": false,
                                    "text": "",
                                    "font_size": 0.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "center_x": false,
                                    "dy": 0.0,
                                    "label_dy": 0.0,
                                    "icon": false,
                                    "glyph": null,
                                    "glyph_pad": 8.0,
                                    "bottom": false,
                                    "glyph_color": null,
                                    "glyph_color_hover": null,
                                    "color": null,
                                    "color_hover": null,
                                    "region_dx": 0.0,
                                    "region_dy": 0.0,
                                    "region_dw": 0.0,
                                    "region_dh": 0.0,
                                    "key": "item.key",
                                    "key_add": 0,
                                    "action": null,
                                    "visible": null
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      },
                      {
                        "tag": "icon_chip_hl",
                        "items": [
                          {
                            "Stack": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": "item.w",
                              "h": 0.0,
                              "pad": 0.0,
                              "visible": null,
                              "auto": false,
                              "items": [
                                {
                                  "Surface": {
                                    "x": 0.0,
                                    "y": 8.0,
                                    "w": 22.0,
                                    "h": 22.0,
                                    "r": 11.0,
                                    "r_var": null,
                                    "visible": null,
                                    "hover_key": "item.key",
                                    "hover_key_var": null,
                                    "hover_key_add": 0,
                                    "color": "hover_hl",
                                    "hover_color": null,
                                    "right": false,
                                    "center_x": false,
                                    "center_y": false,
                                    "bottom": false,
                                    "tr": 0.0,
                                    "br": 0.0,
                                    "bl": 0.0,
                                    "stroke": 0.0,
                                    "stroke_color": null
                                  }
                                },
                                {
                                  "Hit": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 0.0,
                                    "h": 0.0,
                                    "anchors": {
                                      "left": -2.0,
                                      "top": 4.0,
                                      "right": 2.0,
                                      "bottom": -4.0
                                    },
                                    "r": 0.0,
                                    "surface": null,
                                    "surface_hover": null,
                                    "hover_only": false,
                                    "text": "",
                                    "font_size": 0.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "center_x": false,
                                    "dy": 0.0,
                                    "label_dy": 0.0,
                                    "icon": false,
                                    "glyph": null,
                                    "glyph_pad": 8.0,
                                    "bottom": false,
                                    "glyph_color": null,
                                    "glyph_color_hover": null,
                                    "color": null,
                                    "color_hover": null,
                                    "region_dx": 0.0,
                                    "region_dy": 0.0,
                                    "region_dw": 0.0,
                                    "region_dh": 0.0,
                                    "key": "item.key",
                                    "key_add": 0,
                                    "action": null,
                                    "visible": null
                                  }
                                },
                                {
                                  "Text": {
                                    "x": 4.0,
                                    "y": "pill_mid",
                                    "w": 0.0,
                                    "h": 0.0,
                                    "text": "{item.glyph}",
                                    "font_size": 14.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "icon": true,
                                    "color": {
                                      "value": "item.ink"
                                    },
                                    "center_x": false,
                                    "center_y": false,
                                    "cy_pct": 0.0,
                                    "cx_pct": 0.0,
                                    "bottom": false,
                                    "visible": null,
                                    "size_ladder": [],
                                    "y_ladder": [],
                                    "dy": 0.0,
                                    "auto": false
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      },
                      {
                        "tag": "watts",
                        "items": [
                          {
                            "Stack": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": "pill_watts_w",
                              "h": 0.0,
                              "pad": 0.0,
                              "visible": null,
                              "auto": false,
                              "items": [
                                {
                                  "Text": {
                                    "x": 0.0,
                                    "y": "pill_mid",
                                    "w": 0.0,
                                    "h": 0.0,
                                    "text": "{item.glyph}",
                                    "font_size": 14.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "icon": true,
                                    "color": {
                                      "value": "item.ink"
                                    },
                                    "center_x": false,
                                    "center_y": false,
                                    "cy_pct": 0.0,
                                    "cx_pct": 0.0,
                                    "bottom": false,
                                    "visible": null,
                                    "size_ladder": [],
                                    "y_ladder": [],
                                    "dy": 0.0,
                                    "auto": false
                                  }
                                },
                                {
                                  "Text": {
                                    "x": 14.0,
                                    "y": "pill_mid",
                                    "w": 0.0,
                                    "h": 0.0,
                                    "text": "{pill_watts}",
                                    "font_size": 14.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "icon": false,
                                    "color": {
                                      "value": "item.ink"
                                    },
                                    "center_x": false,
                                    "center_y": false,
                                    "cy_pct": 0.0,
                                    "cx_pct": 0.0,
                                    "bottom": false,
                                    "visible": null,
                                    "size_ladder": [],
                                    "y_ladder": [],
                                    "dy": 0.0,
                                    "auto": false
                                  }
                                },
                                {
                                  "Hit": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 0.0,
                                    "h": 0.0,
                                    "anchors": {
                                      "left": -2.0,
                                      "top": 4.0,
                                      "right": 2.0,
                                      "bottom": -4.0
                                    },
                                    "r": 0.0,
                                    "surface": null,
                                    "surface_hover": null,
                                    "hover_only": false,
                                    "text": "",
                                    "font_size": 0.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "center_x": false,
                                    "dy": 0.0,
                                    "label_dy": 0.0,
                                    "icon": false,
                                    "glyph": null,
                                    "glyph_pad": 8.0,
                                    "bottom": false,
                                    "glyph_color": null,
                                    "glyph_color_hover": null,
                                    "color": null,
                                    "color_hover": null,
                                    "region_dx": 0.0,
                                    "region_dy": 0.0,
                                    "region_dw": 0.0,
                                    "region_dh": 0.0,
                                    "key": "item.key",
                                    "key_add": 0,
                                    "action": null,
                                    "visible": null
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      },
                      {
                        "tag": "viz",
                        "items": [
                          {
                            "Stack": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 24.0,
                              "h": 0.0,
                              "pad": 0.0,
                              "visible": null,
                              "auto": false,
                              "items": [
                                {
                                  "Spectrum": {
                                    "name": "pill_viz",
                                    "x": -0.5,
                                    "y": "pill_viz_y",
                                    "w": 24.0,
                                    "h": 14.0,
                                    "min_w": 0.0,
                                    "min_h": 0.0,
                                    "kind": "block",
                                    "floor": 0.5,
                                    "bar_frac": 0.0,
                                    "inset": 0.5,
                                    "min_bar_w": 0.0,
                                    "color": "acc",
                                    "accent_every": 4,
                                    "tint": 0.35,
                                    "thickness": 0.0,
                                    "visible": null
                                  }
                                },
                                {
                                  "Hit": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 0.0,
                                    "h": 0.0,
                                    "anchors": {
                                      "left": -2.0,
                                      "top": 4.0,
                                      "right": 2.0,
                                      "bottom": -4.0
                                    },
                                    "r": 0.0,
                                    "surface": null,
                                    "surface_hover": null,
                                    "hover_only": false,
                                    "text": "",
                                    "font_size": 0.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "center_x": false,
                                    "dy": 0.0,
                                    "label_dy": 0.0,
                                    "icon": false,
                                    "glyph": null,
                                    "glyph_pad": 8.0,
                                    "bottom": false,
                                    "glyph_color": null,
                                    "glyph_color_hover": null,
                                    "color": null,
                                    "color_hover": null,
                                    "region_dx": 0.0,
                                    "region_dy": 0.0,
                                    "region_dw": 0.0,
                                    "region_dh": 0.0,
                                    "key": "item.key",
                                    "key_add": 0,
                                    "action": null,
                                    "visible": null
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      },
                      {
                        "tag": "user",
                        "items": [
                          {
                            "Stack": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": "pill_user_w",
                              "h": 0.0,
                              "pad": 0.0,
                              "visible": null,
                              "auto": false,
                              "items": [
                                {
                                  "Text": {
                                    "x": 0.0,
                                    "y": "pill_mid",
                                    "w": 0.0,
                                    "h": 0.0,
                                    "text": "{username}",
                                    "font_size": 14.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "icon": false,
                                    "color": {
                                      "value": "item.ink"
                                    },
                                    "center_x": false,
                                    "center_y": false,
                                    "cy_pct": 0.0,
                                    "cx_pct": 0.0,
                                    "bottom": false,
                                    "visible": null,
                                    "size_ladder": [],
                                    "y_ladder": [],
                                    "dy": 0.0,
                                    "auto": false
                                  }
                                },
                                {
                                  "Hit": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 0.0,
                                    "h": 0.0,
                                    "anchors": {
                                      "left": -2.0,
                                      "top": 4.0,
                                      "right": 2.0,
                                      "bottom": -4.0
                                    },
                                    "r": 0.0,
                                    "surface": null,
                                    "surface_hover": null,
                                    "hover_only": false,
                                    "text": "",
                                    "font_size": 0.0,
                                    "font": "ui",
                                    "weight": "light",
                                    "right": false,
                                    "center": false,
                                    "center_x": false,
                                    "dy": 0.0,
                                    "label_dy": 0.0,
                                    "icon": false,
                                    "glyph": null,
                                    "glyph_pad": 8.0,
                                    "bottom": false,
                                    "glyph_color": null,
                                    "glyph_color_hover": null,
                                    "color": null,
                                    "color_hover": null,
                                    "region_dx": 0.0,
                                    "region_dy": 0.0,
                                    "region_dw": 0.0,
                                    "region_dh": 0.0,
                                    "key": "item.key",
                                    "key_add": 0,
                                    "action": null,
                                    "visible": null
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      },
                      {
                        "tag": "branding",
                        "items": [
                          {
                            "Stack": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": "pill_brand_w",
                              "h": 0.0,
                              "pad": 0.0,
                              "visible": null,
                              "auto": false,
                              "items": [
                                {
                                  "Text": {
                                    "x": 0.0,
                                    "y": "pill_mid",
                                    "w": 0.0,
                                    "h": 0.0,
                                    "text": "{pill_brand_glyph}",
                                    "font_size": 16.0,
                                    "font": "brand",
                                    "weight": "regular",
                                    "right": false,
                                    "center": true,
                                    "icon": false,
                                    "color": "fg",
                                    "center_x": false,
                                    "center_y": false,
                                    "cy_pct": 0.0,
                                    "cx_pct": 0.0,
                                    "bottom": false,
                                    "visible": null,
                                    "size_ladder": [],
                                    "y_ladder": [],
                                    "dy": 0.0,
                                    "auto": false
                                  }
                                }
                              ]
                            }
                          }
                        ]
                      }
                    ]
                  }
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 0.0,
              "y": 0.0,
              "w": "pill_right_w",
              "h": 0.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": "pill_cc_key",
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                },
                {
                  "Row": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "pad": 0.0,
                    "spacing": 6.0,
                    "valign": "top",
                    "halign": "left",
                    "bottom": false,
                    "min_card_h": 0.0,
                    "visible": null,
                    "auto": true,
                    "items": [
                      {
                        "Stack": {
                          "x": 0.0,
                          "y": 0.0,
                          "w": 22.0,
                          "h": 0.0,
                          "pad": 0.0,
                          "visible": "pill_bell_on",
                          "auto": false,
                          "items": [
                            {
                              "Surface": {
                                "x": 0.0,
                                "y": 8.0,
                                "w": "parent.w",
                                "h": 22.0,
                                "r": 11.0,
                                "r_var": null,
                                "visible": null,
                                "hover_key": null,
                                "hover_key_var": "pill_bell_key",
                                "hover_key_add": 0,
                                "color": "hover",
                                "hover_color": null,
                                "right": false,
                                "center_x": false,
                                "center_y": false,
                                "bottom": false,
                                "tr": 0.0,
                                "br": 0.0,
                                "bl": 0.0,
                                "stroke": 0.0,
                                "stroke_color": null
                              }
                            },
                            {
                              "Hit": {
                                "x": 0.0,
                                "y": 0.0,
                                "w": "parent.w",
                                "h": 0.0,
                                "anchors": null,
                                "r": 0.0,
                                "surface": null,
                                "surface_hover": null,
                                "hover_only": false,
                                "text": "",
                                "font_size": 0.0,
                                "font": "ui",
                                "weight": "light",
                                "right": false,
                                "center": false,
                                "center_x": false,
                                "dy": 0.0,
                                "label_dy": 0.0,
                                "icon": false,
                                "glyph": null,
                                "glyph_pad": 8.0,
                                "bottom": false,
                                "glyph_color": null,
                                "glyph_color_hover": null,
                                "color": null,
                                "color_hover": null,
                                "region_dx": 0.0,
                                "region_dy": 0.0,
                                "region_dw": 0.0,
                                "region_dh": 0.0,
                                "key": "pill_bell_key",
                                "key_add": 0,
                                "action": null,
                                "visible": null
                              }
                            },
                            {
                              "Text": {
                                "x": 0.0,
                                "y": "pill_mid",
                                "w": "parent.w",
                                "h": 0.0,
                                "text": "",
                                "font_size": 14.0,
                                "font": "ui",
                                "weight": "light",
                                "right": true,
                                "center": false,
                                "icon": true,
                                "color": "fg",
                                "center_x": false,
                                "center_y": false,
                                "cy_pct": 0.0,
                                "cx_pct": 0.0,
                                "bottom": false,
                                "visible": null,
                                "size_ladder": [],
                                "y_ladder": [],
                                "dy": 0.0,
                                "auto": false
                              }
                            },
                            {
                              "Surface": {
                                "x": 2.0,
                                "y": 5.0,
                                "w": 6.0,
                                "h": 6.0,
                                "r": 3.0,
                                "r_var": null,
                                "visible": "pill_notif_dot",
                                "hover_key": null,
                                "hover_key_var": null,
                                "hover_key_add": 0,
                                "color": "acc",
                                "hover_color": null,
                                "right": false,
                                "center_x": false,
                                "center_y": false,
                                "bottom": false,
                                "tr": 0.0,
                                "br": 0.0,
                                "bl": 0.0,
                                "stroke": 0.0,
                                "stroke_color": null
                              }
                            }
                          ]
                        }
                      },
                      {
                        "Stack": {
                          "x": 0.0,
                          "y": 0.0,
                          "w": 22.0,
                          "h": 0.0,
                          "pad": 0.0,
                          "visible": "pill_bright_on",
                          "auto": false,
                          "items": [
                            {
                              "Surface": {
                                "x": 0.0,
                                "y": 8.0,
                                "w": "parent.w",
                                "h": 22.0,
                                "r": 11.0,
                                "r_var": null,
                                "visible": null,
                                "hover_key": null,
                                "hover_key_var": "pill_bright_key",
                                "hover_key_add": 0,
                                "color": "hover_hl",
                                "hover_color": null,
                                "right": false,
                                "center_x": false,
                                "center_y": false,
                                "bottom": false,
                                "tr": 0.0,
                                "br": 0.0,
                                "bl": 0.0,
                                "stroke": 0.0,
                                "stroke_color": null
                              }
                            },
                            {
                              "Hit": {
                                "x": 0.0,
                                "y": 0.0,
                                "w": "parent.w",
                                "h": 0.0,
                                "anchors": null,
                                "r": 0.0,
                                "surface": null,
                                "surface_hover": null,
                                "hover_only": false,
                                "text": "",
                                "font_size": 0.0,
                                "font": "ui",
                                "weight": "light",
                                "right": false,
                                "center": false,
                                "center_x": false,
                                "dy": 0.0,
                                "label_dy": 0.0,
                                "icon": false,
                                "glyph": null,
                                "glyph_pad": 8.0,
                                "bottom": false,
                                "glyph_color": null,
                                "glyph_color_hover": null,
                                "color": null,
                                "color_hover": null,
                                "region_dx": 0.0,
                                "region_dy": 0.0,
                                "region_dw": 0.0,
                                "region_dh": 0.0,
                                "key": "pill_bright_key",
                                "key_add": 0,
                                "action": null,
                                "visible": null
                              }
                            },
                            {
                              "Text": {
                                "x": 5.0,
                                "y": "pill_mid",
                                "w": 0.0,
                                "h": 0.0,
                                "text": "{pill_bright_glyph}",
                                "font_size": 14.0,
                                "font": "ui",
                                "weight": "light",
                                "right": false,
                                "center": false,
                                "icon": true,
                                "color": {
                                  "value": "pill_bright_ink"
                                },
                                "center_x": false,
                                "center_y": false,
                                "cy_pct": 0.0,
                                "cx_pct": 0.0,
                                "bottom": false,
                                "visible": null,
                                "size_ladder": [],
                                "y_ladder": [],
                                "dy": 0.0,
                                "auto": false
                              }
                            }
                          ]
                        }
                      },
                      {
                        "Repeat": {
                          "name": "pill_tray",
                          "model": null,
                          "x": 0.0,
                          "y": 0.0,
                          "w": 0.0,
                          "h": 0.0,
                          "pad": 0.0,
                          "spacing": 0.0,
                          "visible": "pill_tray_on",
                          "auto": true,
                          "dir": "across",
                          "slot_key": 65,
                          "templates": [
                            {
                              "tag": "*",
                              "items": [
                                {
                                  "Stack": {
                                    "x": 0.0,
                                    "y": 0.0,
                                    "w": 22.0,
                                    "h": 0.0,
                                    "pad": 0.0,
                                    "visible": null,
                                    "auto": false,
                                    "items": [
                                      {
                                        "Image": {
                                          "x": 2.0,
                                          "y": 11.0,
                                          "w": 16.0,
                                          "h": 16.0,
                                          "key": "{slot}",
                                          "fallback": {
                                            "Text": {
                                              "x": 6.5,
                                              "y": 13.5,
                                              "w": 0.0,
                                              "h": 0.0,
                                              "text": "",
                                              "font_size": 8.0,
                                              "font": "ui",
                                              "weight": "light",
                                              "right": false,
                                              "center": false,
                                              "icon": true,
                                              "color": "fg2",
                                              "center_x": false,
                                              "center_y": false,
                                              "cy_pct": 0.0,
                                              "cx_pct": 0.0,
                                              "bottom": false,
                                              "visible": null,
                                              "size_ladder": [],
                                              "y_ladder": [],
                                              "dy": 0.0,
                                              "auto": false
                                            }
                                          },
                                          "right": false,
                                          "visible": null
                                        }
                                      },
                                      {
                                        "Hit": {
                                          "x": 0.0,
                                          "y": 8.0,
                                          "w": 20.0,
                                          "h": 22.0,
                                          "anchors": null,
                                          "r": 0.0,
                                          "surface": null,
                                          "surface_hover": null,
                                          "hover_only": false,
                                          "text": "",
                                          "font_size": 0.0,
                                          "font": "ui",
                                          "weight": "light",
                                          "right": false,
                                          "center": false,
                                          "center_x": false,
                                          "dy": 0.0,
                                          "label_dy": 0.0,
                                          "icon": false,
                                          "glyph": null,
                                          "glyph_pad": 8.0,
                                          "bottom": false,
                                          "glyph_color": null,
                                          "glyph_color_hover": null,
                                          "color": null,
                                          "color_hover": null,
                                          "region_dx": 0.0,
                                          "region_dy": 0.0,
                                          "region_dw": 0.0,
                                          "region_dh": 0.0,
                                          "key": 0.0,
                                          "key_add": 0,
                                          "action": null,
                                          "visible": null
                                        }
                                      },
                                      {
                                        "Surface": {
                                          "x": 0.0,
                                          "y": 9.0,
                                          "w": 20.0,
                                          "h": 20.0,
                                          "r": 10.0,
                                          "r_var": null,
                                          "visible": null,
                                          "hover_key": 0.0,
                                          "hover_key_var": null,
                                          "hover_key_add": 0,
                                          "color": "hover_hl",
                                          "hover_color": null,
                                          "right": false,
                                          "center_x": false,
                                          "center_y": false,
                                          "bottom": false,
                                          "tr": 0.0,
                                          "br": 0.0,
                                          "bl": 0.0,
                                          "stroke": 0.0,
                                          "stroke_color": null
                                        }
                                      }
                                    ]
                                  }
                                }
                              ]
                            }
                          ]
                        }
                      }
                    ]
                  }
                }
              ]
            }
          }
        ]
      }
    }
  ]
}"#;

const POLKIT_AUTH_SCENE_JSON: &str = r#"{
  "name": "polkit_auth",
  "stores": [],
  "items": [
    {
      "Text": {
        "x": 0.0,
        "y": 32.0,
        "w": 48.0,
        "h": 0.0,
        "text": "",
        "font_size": 36.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": true,
        "color": "acc",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.5,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": 88.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{polkit_msg}",
        "font_size": 13.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": true,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": 108.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{polkit_act}",
        "font_size": 10.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg3",
        "center_x": true,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": 136.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{polkit_user}",
        "font_size": 14.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": true,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Stack": {
        "x": 24.0,
        "y": 164.0,
        "w": -24.0,
        "h": 36.0,
        "pad": 0.0,
        "visible": null,
        "auto": false,
        "items": [
          {
            "Surface": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "r": 8.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "polkit_pw_bg"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Hit": {
              "x": 0.0,
              "y": 0.0,
              "w": 0.0,
              "h": 0.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 92.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          }
        ]
      }
    },
    {
      "Text": {
        "x": 36.0,
        "y": 174.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{polkit_pw_disp}",
        "font_size": 13.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": {
          "value": "polkit_pw_ink"
        },
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Surface": {
        "x": {
          "val": "polkit_cursor_x",
          "anim": {
            "ms": 90.0,
            "curve": "out_cubic"
          }
        },
        "y": 172.0,
        "w": 2.0,
        "h": 20.0,
        "r": 1.0,
        "r_var": null,
        "visible": "polkit_cursor_on",
        "hover_key": null,
        "hover_key_var": null,
        "hover_key_add": 0,
        "color": "fg",
        "hover_color": null,
        "right": false,
        "center_x": false,
        "center_y": false,
        "bottom": false,
        "tr": 0.0,
        "br": 0.0,
        "bl": 0.0,
        "stroke": 0.0,
        "stroke_color": null
      }
    },
    {
      "Text": {
        "x": 0.0,
        "y": 208.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{polkit_err}",
        "font_size": 11.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": {
          "value": "polkit_err_ink"
        },
        "center_x": true,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "polkit_err_on",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Row": {
        "x": 0.0,
        "y": 24.0,
        "w": 0.0,
        "h": 36.0,
        "pad": 0.0,
        "spacing": 16.0,
        "valign": "top",
        "halign": "center",
        "bottom": true,
        "min_card_h": 0.0,
        "visible": null,
        "auto": false,
        "items": [
          {
            "Stack": {
              "x": 0.0,
              "y": 0.0,
              "w": 120.0,
              "h": 36.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "r": 8.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "polkit_cancel_bg"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Text": {
                    "x": 0.0,
                    "y": 10.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "Cancel",
                    "font_size": 13.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": true,
                    "icon": false,
                    "color": "fg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 91.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 0.0,
              "y": 0.0,
              "w": 120.0,
              "h": 36.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "r": 8.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "polkit_auth_bg"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Text": {
                    "x": 0.0,
                    "y": 10.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "{polkit_auth_label}",
                    "font_size": 13.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": true,
                    "icon": false,
                    "color": "sfg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 90.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          }
        ]
      }
    }
  ]
}"#;

const POWER_SCENE_JSON: &str = r#"{
  "name": "power",
  "stores": [],
  "items": [
    {
      "Text": {
        "x": 22.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "Power",
        "font_size": 15.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Row": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "pad": 0.0,
        "spacing": 14.0,
        "valign": "middle",
        "halign": "center",
        "bottom": false,
        "min_card_h": 0.0,
        "visible": null,
        "auto": false,
        "items": [
          {
            "Stack": {
              "x": 0.0,
              "y": 0.0,
              "w": 92.0,
              "h": 92.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "r": 18.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "ppower_bg_0"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Bar": {
                    "x": 2.0,
                    "y": -2.0,
                    "w": "parent.w - 4",
                    "h": "parent.h",
                    "r": 16.0,
                    "max": 1.0,
                    "value": "{ppower_hold_0}",
                    "fill": {
                      "value": "power_hold_fill"
                    },
                    "track": {
                      "value": "clear"
                    },
                    "reserve": 0.0,
                    "vertical": true,
                    "visible": "ppower_hold_on_0"
                  }
                },
                {
                  "Text": {
                    "x": 34.0,
                    "y": 24.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 24.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": {
                      "value": "ppower_ink_0"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 0.0,
                    "y": 60.0,
                    "w": "parent.w",
                    "h": 0.0,
                    "text": "Lock",
                    "font_size": 12.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": true,
                    "icon": false,
                    "color": "fg2",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 1.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 0.0,
              "y": 0.0,
              "w": 92.0,
              "h": 92.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "r": 18.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "ppower_bg_1"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Bar": {
                    "x": 2.0,
                    "y": -2.0,
                    "w": "parent.w - 4",
                    "h": "parent.h",
                    "r": 16.0,
                    "max": 1.0,
                    "value": "{ppower_hold_1}",
                    "fill": {
                      "value": "power_hold_fill"
                    },
                    "track": {
                      "value": "clear"
                    },
                    "reserve": 0.0,
                    "vertical": true,
                    "visible": "ppower_hold_on_1"
                  }
                },
                {
                  "Text": {
                    "x": 34.0,
                    "y": 24.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 24.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": {
                      "value": "ppower_ink_1"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 0.0,
                    "y": 60.0,
                    "w": "parent.w",
                    "h": 0.0,
                    "text": "Logout",
                    "font_size": 12.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": true,
                    "icon": false,
                    "color": "fg2",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 2.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 0.0,
              "y": 0.0,
              "w": 92.0,
              "h": 92.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "r": 18.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "ppower_bg_2"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Bar": {
                    "x": 2.0,
                    "y": -2.0,
                    "w": "parent.w - 4",
                    "h": "parent.h",
                    "r": 16.0,
                    "max": 1.0,
                    "value": "{ppower_hold_2}",
                    "fill": {
                      "value": "power_hold_fill"
                    },
                    "track": {
                      "value": "clear"
                    },
                    "reserve": 0.0,
                    "vertical": true,
                    "visible": "ppower_hold_on_2"
                  }
                },
                {
                  "Text": {
                    "x": 34.0,
                    "y": 24.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 24.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": {
                      "value": "ppower_ink_2"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 0.0,
                    "y": 60.0,
                    "w": "parent.w",
                    "h": 0.0,
                    "text": "Sleep",
                    "font_size": 12.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": true,
                    "icon": false,
                    "color": "fg2",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 3.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 0.0,
              "y": 0.0,
              "w": 92.0,
              "h": 92.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "r": 18.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "ppower_bg_3"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Bar": {
                    "x": 2.0,
                    "y": -2.0,
                    "w": "parent.w - 4",
                    "h": "parent.h",
                    "r": 16.0,
                    "max": 1.0,
                    "value": "{ppower_hold_3}",
                    "fill": {
                      "value": "power_hold_fill"
                    },
                    "track": {
                      "value": "clear"
                    },
                    "reserve": 0.0,
                    "vertical": true,
                    "visible": "ppower_hold_on_3"
                  }
                },
                {
                  "Text": {
                    "x": 34.0,
                    "y": 24.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 24.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": {
                      "value": "ppower_ink_3"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 0.0,
                    "y": 60.0,
                    "w": "parent.w",
                    "h": 0.0,
                    "text": "Restart",
                    "font_size": 12.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": true,
                    "icon": false,
                    "color": "fg2",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 4.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 0.0,
              "y": 0.0,
              "w": 92.0,
              "h": 92.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "r": 18.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "ppower_bg_4"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Bar": {
                    "x": 2.0,
                    "y": -2.0,
                    "w": "parent.w - 4",
                    "h": "parent.h",
                    "r": 16.0,
                    "max": 1.0,
                    "value": "{ppower_hold_4}",
                    "fill": {
                      "value": "power_hold_fill"
                    },
                    "track": {
                      "value": "clear"
                    },
                    "reserve": 0.0,
                    "vertical": true,
                    "visible": "ppower_hold_on_4"
                  }
                },
                {
                  "Text": {
                    "x": 34.0,
                    "y": 24.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "",
                    "font_size": 24.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": {
                      "value": "ppower_ink_4"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 0.0,
                    "y": 60.0,
                    "w": "parent.w",
                    "h": 0.0,
                    "text": "Shutdown",
                    "font_size": 12.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": true,
                    "icon": false,
                    "color": "fg2",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 5.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          }
        ]
      }
    }
  ]
}"#;

const SETTINGS_SCENE_JSON: &str = r#"{
  "name": "settings",
  "stores": [
    {
      "name": "nav",
      "props": {
        "tab": 0.0
      }
    }
  ],
  "items": [
    {
      "Stack": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "pad": 0.0,
        "visible": "tab_misc",
        "auto": false,
        "items": [
          {
            "Text": {
              "x": 188.0,
              "y": 50.0,
              "w": 0.0,
              "h": 0.0,
              "text": "MISC",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": 64.0,
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "acc_chip",
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": 71.0,
              "w": 0.0,
              "h": 0.0,
              "text": "\uf573",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": 64.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Power menu hold delay",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": 82.0,
              "w": 0.0,
              "h": 0.0,
              "text": "{misc_hold_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "misc_hold_fader",
              "x": 244.0,
              "y": 96.0,
              "w": -32.0,
              "h": 10.0,
              "key": 220,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": 55.0,
              "w": -10.0,
              "h": 44.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 220.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          }
        ]
      }
    },
    {
      "Stack": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "pad": 0.0,
        "visible": "tab_system",
        "auto": false,
        "items": [
          {
            "Text": {
              "x": 188.0,
              "y": 50.0,
              "w": 0.0,
              "h": 0.0,
              "text": "SYSTEM",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Surface": {
              "x": 188.0,
              "y": 58.0,
              "w": -8.0,
              "h": 58.0,
              "r": 14.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "raised",
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 1.0,
              "stroke_color": "hairline"
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "sys_rows",
              "x": 188.0,
              "y": 66.0,
              "w": -8.0,
              "h": 14.0,
              "pad": 0.0,
              "spacing": 0.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "stat",
                  "items": [
                    {
                      "Text": {
                        "x": 12.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 9.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": "fg",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Bar": {
                        "x": 54.0,
                        "y": 1.0,
                        "w": -88.0,
                        "h": 5.0,
                        "r": 2.5,
                        "max": 0.0,
                        "value": "{item.pct}",
                        "fill": "acc",
                        "track": "hover",
                        "reserve": 0.0,
                        "vertical": false,
                        "visible": null
                      }
                    },
                    {
                      "Text": {
                        "x": 16.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.pct}%",
                        "font_size": 9.0,
                        "font": "ui",
                        "weight": "light",
                        "right": true,
                        "center": false,
                        "icon": false,
                        "color": "fg3",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Text": {
              "x": 16.0,
              "y": 122.0,
              "w": 0.0,
              "h": 0.0,
              "text": "{net_rate}",
              "font_size": 9.0,
              "font": "ui",
              "weight": "light",
              "right": true,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": "sys_net_live",
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 16.0,
              "y": "parent.h - 20",
              "w": 0.0,
              "h": 0.0,
              "text": "zen-shell v0.1.0",
              "font_size": 9.0,
              "font": "ui",
              "weight": "light",
              "right": true,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          }
        ]
      }
    },
    {
      "Stack": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "pad": 0.0,
        "visible": "tab_sound",
        "auto": false,
        "items": [
          {
            "Text": {
              "x": 188.0,
              "y": 50.0,
              "w": 0.0,
              "h": 0.0,
              "text": "SOUND & DISPLAY",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": 64.0,
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "acc_chip",
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": 71.0,
              "w": 0.0,
              "h": 0.0,
              "text": "\uf185",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": 72.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Brightness",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "sound_brightness",
              "x": 244.0,
              "y": 90.0,
              "w": -32.0,
              "h": 10.0,
              "key": 5,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": 55.0,
              "w": -10.0,
              "h": 44.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 5.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": 108.0,
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "acc_chip",
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": 115.0,
              "w": 0.0,
              "h": 0.0,
              "text": "\uf028",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": 116.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Volume",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "sound_volume",
              "x": 244.0,
              "y": 134.0,
              "w": -32.0,
              "h": 10.0,
              "key": 4,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": 99.0,
              "w": -10.0,
              "h": 44.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 4.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": 152.0,
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "acc_chip",
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": 159.0,
              "w": 0.0,
              "h": 0.0,
              "text": "\uf0b2",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": 160.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Max volume",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Stack": {
              "x": "parent.w - 298.0",
              "y": 154.0,
              "w": 44.0,
              "h": 24.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 0.0,
                    "r": 12.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "sound_over100_fill"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Text": {
                    "x": 0.0,
                    "y": 4.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "{sound_over100_glyph}",
                    "font_size": 10.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": true,
                    "icon": true,
                    "color": "fg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                }
              ]
            }
          },
          {
            "Hit": {
              "x": 250.0,
              "y": 150.0,
              "w": 52.0,
              "h": 32.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": true,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 14.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Fader": {
              "name": "sound_maxvol",
              "x": 244.0,
              "y": 178.0,
              "w": -32.0,
              "h": 10.0,
              "key": 15,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": 153.0,
              "w": -10.0,
              "h": 34.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 15.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Text": {
              "x": 248.0,
              "y": 197.0,
              "w": 0.0,
              "h": 0.0,
              "text": "\uf0ec",
              "font_size": 9.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "sound_balance",
              "x": 244.0,
              "y": 198.0,
              "w": -32.0,
              "h": 10.0,
              "key": 59,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": null,
              "head": 10.0,
              "centered": true,
              "visible": null
            }
          },
          {
            "Text": {
              "x": 36.0,
              "y": 197.0,
              "w": 0.0,
              "h": 0.0,
              "text": "\uf06e",
              "font_size": 9.0,
              "font": "ui",
              "weight": "light",
              "right": true,
              "center": false,
              "icon": true,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": 210.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Balance",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Hit": {
              "x": "parent.w - 24.0",
              "y": 198.0,
              "w": 52.0,
              "h": 22.0,
              "anchors": null,
              "r": 7.0,
              "surface": "chip",
              "surface_hover": "hover_hl",
              "hover_only": false,
              "text": "\uf0e2 Reset",
              "font_size": 9.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": true,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 4.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": "fg",
              "color_hover": "acc",
              "region_dx": 2.0,
              "region_dy": 2.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 58.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 242.0,
              "y": 198.0,
              "w": -30.0,
              "h": 24.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 59.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          }
        ]
      }
    },
    {
      "Stack": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "pad": 0.0,
        "visible": "tab_pill",
        "auto": false,
        "items": [
          {
            "Text": {
              "x": 188.0,
              "y": 50.0,
              "w": 0.0,
              "h": 0.0,
              "text": "PILL",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "pill_so + 64.0",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "acc_chip",
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "pill_so + 71.0",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf303",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "pill_so + 64.0",
              "w": 0.0,
              "h": 0.0,
              "text": "Margin",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "pill_so + 82.0",
              "w": 0.0,
              "h": 0.0,
              "text": "{pill_margin_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "pill_margin",
              "x": 244.0,
              "y": "pill_so + 94.0",
              "w": -32.0,
              "h": 10.0,
              "key": 36,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": "pill_so + 55.0",
              "w": -10.0,
              "h": 44.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 36.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "pill_so + 108.0",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "acc_chip",
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "pill_so + 115.0",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf06e",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "pill_so + 108.0",
              "w": 0.0,
              "h": 0.0,
              "text": "Pill transparency",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "pill_so + 126.0",
              "w": 0.0,
              "h": 0.0,
              "text": "{pill_alpha_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "pill_alpha",
              "x": 244.0,
              "y": "pill_so + 138.0",
              "w": -32.0,
              "h": 10.0,
              "key": 37,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": "pill_so + 99.0",
              "w": -10.0,
              "h": 44.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 37.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "pill_so + 150.0",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": "acc_chip",
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "pill_so + 157.0",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf06e",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "pill_so + 150.0",
              "w": 0.0,
              "h": 0.0,
              "text": "Card transparency",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "pill_so + 168.0",
              "w": 0.0,
              "h": 0.0,
              "text": "{pill_card_alpha_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "pill_card_alpha",
              "x": 244.0,
              "y": "pill_so + 182.0",
              "w": -32.0,
              "h": 10.0,
              "key": 38,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": "pill_so + 141.0",
              "w": -10.0,
              "h": 44.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 38.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "pill_behavior",
              "x": 192.0,
              "y": "196.0 + pill_so",
              "w": -12.0,
              "h": 40.0,
              "pad": 0.0,
              "spacing": 0.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "*",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 40.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": "item.key",
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover_hl",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": 8.0,
                        "y": 8.0,
                        "w": 24.0,
                        "h": 24.0,
                        "r": 6.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 14.0,
                        "y": 14.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 12.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.ink"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 44.0,
                        "y": 14.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 12.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 12.0,
                        "y": 11.0,
                        "w": 36.0,
                        "h": 18.0,
                        "r": 9.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.track"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": "item.on ? 14.0 : 32.0",
                        "y": 13.0,
                        "w": 14.0,
                        "h": 14.0,
                        "r": 7.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.knob"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": -2.0,
                        "w": 0.0,
                        "h": 36.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 8.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "pill_behavior_rest",
              "x": 192.0,
              "y": "236.0 + pill_so + pill_tip",
              "w": -12.0,
              "h": 40.0,
              "pad": 0.0,
              "spacing": 0.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "*",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 40.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": "item.key",
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover_hl",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": 8.0,
                        "y": 8.0,
                        "w": 24.0,
                        "h": 24.0,
                        "r": 6.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 14.0,
                        "y": 14.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 12.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.ink"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 44.0,
                        "y": 14.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 12.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 12.0,
                        "y": 11.0,
                        "w": 36.0,
                        "h": 18.0,
                        "r": 9.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.track"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": "item.on ? 14.0 : 32.0",
                        "y": 13.0,
                        "w": 14.0,
                        "h": 14.0,
                        "r": 7.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.knob"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": -2.0,
                        "w": 0.0,
                        "h": 36.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 8.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 192.0,
              "y": "236.0 + pill_so",
              "w": -12.0,
              "h": 44.0,
              "pad": 0.0,
              "visible": "pill_tip_live",
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 44.0,
                    "r": 10.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": "acc_tint",
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Text": {
                    "x": 12.0,
                    "y": 14.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "\uf05a",
                    "font_size": 14.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": "acc",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 36.0,
                    "y": 7.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "Expand on hover is ON",
                    "font_size": 10.5,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": false,
                    "color": "fg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 36.0,
                    "y": 24.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "Adding clickable items to the resting pill is discouraged.",
                    "font_size": 10.5,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": false,
                    "color": "fg2",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                }
              ]
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": "404.0 + pill_so + pill_tip",
              "w": 0.0,
              "h": 0.0,
              "text": "PILL ITEMS \u00b7 hold & drag to reorder",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "pill_items",
              "x": 192.0,
              "y": "422.0 + pill_so + pill_tip",
              "w": -12.0,
              "h": 40.0,
              "pad": 0.0,
              "spacing": 0.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "row",
                  "items": [
                    {
                      "Text": {
                        "x": -10.0,
                        "y": 10.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "\uf5d4",
                        "font_size": 14.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.gink"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 40.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": "item.key",
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover_hl",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": 8.0,
                        "y": 8.0,
                        "w": 24.0,
                        "h": 24.0,
                        "r": 6.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 14.0,
                        "y": 14.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 12.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.ink"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 44.0,
                        "y": 14.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 12.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 12.0,
                        "y": 11.0,
                        "w": 36.0,
                        "h": 18.0,
                        "r": 9.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.track"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": "item.on ? 14.0 : 32.0",
                        "y": 13.0,
                        "w": 14.0,
                        "h": 14.0,
                        "r": 7.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.knob"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": -2.0,
                        "w": 0.0,
                        "h": 36.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 8.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                },
                {
                  "tag": "lifted",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 36.0,
                        "r": 10.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "pill_lifted_fill"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Stack": {
              "x": "clamp(pill_drag_px - (parent.w - 204.0) / 2.0, 188.0, 196.0)",
              "y": "pill_drag_py - 19.0",
              "w": "parent.w - 204.0",
              "h": 40.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Repeat": {
                    "name": "",
                    "model": "pill_ghost",
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 40.0,
                    "pad": 0.0,
                    "spacing": 0.0,
                    "visible": null,
                    "auto": false,
                    "dir": "down",
                    "slot_key": null,
                    "templates": [
                      {
                        "tag": "*",
                        "items": [
                          {
                            "Surface": {
                              "x": 8.0,
                              "y": 8.0,
                              "w": 24.0,
                              "h": 24.0,
                              "r": 6.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": "hover",
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Text": {
                              "x": 14.0,
                              "y": 14.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "{item.glyph}",
                              "font_size": 12.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": true,
                              "color": {
                                "value": "item.ink"
                              },
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Text": {
                              "x": 44.0,
                              "y": 14.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "{item.label}",
                              "font_size": 12.5,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": false,
                              "color": {
                                "value": "item.lab"
                              },
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Surface": {
                              "x": 12.0,
                              "y": 11.0,
                              "w": 36.0,
                              "h": 18.0,
                              "r": 9.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.track"
                              },
                              "hover_color": null,
                              "right": true,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Surface": {
                              "x": "item.on ? 14.0 : 32.0",
                              "y": 13.0,
                              "w": 14.0,
                              "h": 14.0,
                              "r": 7.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.knob"
                              },
                              "hover_color": null,
                              "right": true,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          }
                        ]
                      }
                    ]
                  }
                }
              ]
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "pill_batt",
              "x": 192.0,
              "y": "422.0 + pill_so + 680.0 + pill_tip",
              "w": -12.0,
              "h": 40.0,
              "pad": 0.0,
              "spacing": 0.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "*",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 40.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": "item.key",
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover_hl",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": 8.0,
                        "y": 8.0,
                        "w": 24.0,
                        "h": 24.0,
                        "r": 6.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 14.0,
                        "y": 14.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 12.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.ink"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 44.0,
                        "y": 14.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 12.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 12.0,
                        "y": 11.0,
                        "w": 36.0,
                        "h": 18.0,
                        "r": 9.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.track"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": "item.on ? 14.0 : 32.0",
                        "y": 13.0,
                        "w": 14.0,
                        "h": 14.0,
                        "r": 7.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.knob"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": -2.0,
                        "w": 0.0,
                        "h": 36.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 8.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "1156.0 + pill_so + pill_tip",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "pill_pos_tile"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "1163.0 + pill_so + pill_tip",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf0b2",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "fg2",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "1163.0 + pill_so + pill_tip",
              "w": 0.0,
              "h": 0.0,
              "text": "Position",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "pill_edges",
              "x": 320.0,
              "y": "1156.0 + pill_so + pill_tip",
              "w": 0.0,
              "h": 0.0,
              "pad": 0.0,
              "spacing": 6.0,
              "visible": null,
              "auto": true,
              "dir": "across",
              "slot_key": null,
              "templates": [
                {
                  "tag": "*",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 34.0,
                        "h": 30.0,
                        "r": 15.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.bg"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 0.0,
                        "y": 10.0,
                        "w": 34.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 12.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": true,
                        "icon": true,
                        "color": {
                          "value": "item.ink"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 34.0,
                        "h": 30.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          }
        ]
      }
    },
    {
      "Stack": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "pad": 0.0,
        "visible": "tab_appearance",
        "auto": false,
        "items": [
          {
            "Text": {
              "x": 188.0,
              "y": 50.0,
              "w": 0.0,
              "h": 0.0,
              "text": "APPEARANCE",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": 70.0,
              "w": 0.0,
              "h": 0.0,
              "text": "Accent color",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Surface": {
              "x": 30.0,
              "y": 70.0,
              "w": 14.0,
              "h": 14.0,
              "r": 7.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "app_cur"
              },
              "hover_color": null,
              "right": true,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 52.0,
              "y": 71.0,
              "w": 0.0,
              "h": 0.0,
              "text": "{app_cur_hex}",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": true,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": "118.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "ACCENT TOGGLES",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "app_toggles",
              "x": 188.0,
              "y": "140.0 + app_so",
              "w": -8.0,
              "h": 48.0,
              "pad": 0.0,
              "spacing": 4.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "*",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 48.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": "item.key",
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover_hl",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": 8.0,
                        "y": 9.0,
                        "w": 30.0,
                        "h": 30.0,
                        "r": 7.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 15.0,
                        "y": 17.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 14.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.gly"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 48.0,
                        "y": 16.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 13.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": "fg",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 16.0,
                        "y": 14.0,
                        "w": 38.0,
                        "h": 20.0,
                        "r": 10.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.track"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": "item.on ? 17.0 : 36.0",
                        "y": 16.0,
                        "w": 16.0,
                        "h": 16.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.knob"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 48.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "400.0 + app_so",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "app_ic_fill"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "407.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf004",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "402.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "Start icon tone",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "420.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "{app_tone_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "app_tone_fader",
              "x": 244.0,
              "y": "425.0 + app_so",
              "w": -32.0,
              "h": 10.0,
              "key": 211,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": "400.0 + app_so",
              "w": -10.0,
              "h": 40.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 211.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": "452.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "ALPHA",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "474.0 + app_so",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "app_ic_fill"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "481.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf303",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "476.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "Background",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "494.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "{app_alpha0_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "app_alpha0_fader",
              "x": 244.0,
              "y": "499.0 + app_so",
              "w": -32.0,
              "h": 10.0,
              "key": 212,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": "474.0 + app_so",
              "w": -10.0,
              "h": 40.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 212.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "518.0 + app_so",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "app_ic_fill"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "525.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf043",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "520.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "Accent",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "538.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "{app_alpha1_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "app_alpha1_fader",
              "x": 244.0,
              "y": "543.0 + app_so",
              "w": -32.0,
              "h": 10.0,
              "key": 213,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": "518.0 + app_so",
              "w": -10.0,
              "h": 40.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 213.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "562.0 + app_so",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "app_ic_fill"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "569.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf2d0",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "564.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "Scrim",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "582.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "{app_alpha2_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "app_alpha2_fader",
              "x": 244.0,
              "y": "587.0 + app_so",
              "w": -32.0,
              "h": 10.0,
              "key": 214,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": "562.0 + app_so",
              "w": -10.0,
              "h": 40.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 214.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Surface": {
              "x": 204.0,
              "y": "606.0 + app_so",
              "w": 30.0,
              "h": 30.0,
              "r": 9.0,
              "r_var": null,
              "visible": null,
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "app_ic_fill"
              },
              "hover_color": null,
              "right": false,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 211.0,
              "y": "613.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "\uf0b2",
              "font_size": 14.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "acc",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "608.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "Border",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 244.0,
              "y": "626.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "{app_alpha3_lbl}",
              "font_size": 10.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Fader": {
              "name": "app_alpha3_fader",
              "x": 244.0,
              "y": "631.0 + app_so",
              "w": -32.0,
              "h": 10.0,
              "key": 215,
              "action": null,
              "fill": "acc",
              "track": "groove",
              "track_hover": "groove_hl",
              "head": 12.0,
              "centered": false,
              "visible": null
            }
          },
          {
            "Hit": {
              "x": 190.0,
              "y": "606.0 + app_so",
              "w": -10.0,
              "h": 40.0,
              "anchors": null,
              "r": 0.0,
              "surface": null,
              "surface_hover": null,
              "hover_only": false,
              "text": "",
              "font_size": 0.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "center_x": false,
              "dy": 0.0,
              "label_dy": 0.0,
              "icon": false,
              "glyph": null,
              "glyph_pad": 8.0,
              "bottom": false,
              "glyph_color": null,
              "glyph_color_hover": null,
              "color": null,
              "color_hover": null,
              "region_dx": 0.0,
              "region_dy": 0.0,
              "region_dw": 0.0,
              "region_dh": 0.0,
              "key": 215.0,
              "key_add": 0,
              "action": null,
              "visible": null
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "app_scrim",
              "x": 188.0,
              "y": "650.0 + app_so",
              "w": -8.0,
              "h": 48.0,
              "pad": 0.0,
              "spacing": 0.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "*",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 48.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": "item.key",
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover_hl",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": 8.0,
                        "y": 9.0,
                        "w": 30.0,
                        "h": 30.0,
                        "r": 7.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "hover",
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 15.0,
                        "y": 17.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 14.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.gly"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 48.0,
                        "y": 9.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 13.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": "fg",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 48.0,
                        "y": 27.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "overlay on wallpaper",
                        "font_size": 10.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": "fg3",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 16.0,
                        "y": 14.0,
                        "w": 38.0,
                        "h": 20.0,
                        "r": 10.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.track"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": "item.on ? 17.0 : 36.0",
                        "y": 16.0,
                        "w": 16.0,
                        "h": 16.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.knob"
                        },
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 48.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": "702.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "ACCENT SOURCE",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "app_rows",
              "x": 188.0,
              "y": "724.0 + app_so",
              "w": -8.0,
              "h": 38.0,
              "pad": 0.0,
              "spacing": 12.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "row",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 38.0,
                        "r": 10.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.fill"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 16.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 13.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.gly"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 42.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 12.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 38.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                },
                {
                  "tag": "rowsel",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 38.0,
                        "r": 10.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.fill"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 16.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 13.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.gly"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 42.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 12.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 16.0,
                        "y": 13.0,
                        "w": 10.0,
                        "h": 10.0,
                        "r": 5.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "sfg",
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 38.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "app_saved_rows",
              "x": 188.0,
              "y": "1034.0 + app_so",
              "w": -8.0,
              "h": 34.0,
              "pad": 0.0,
              "spacing": 10.0,
              "visible": "app_saved_open",
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "*",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 34.0,
                        "r": 8.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.fill"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Surface": {
                        "x": 12.0,
                        "y": 8.0,
                        "w": 18.0,
                        "h": 18.0,
                        "r": 9.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.col"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 42.0,
                        "y": 9.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.name}",
                        "font_size": 12.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 18.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.hint}",
                        "font_size": 9.5,
                        "font": "ui",
                        "weight": "light",
                        "right": true,
                        "center": false,
                        "icon": false,
                        "color": "fg3",
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 34.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": "1031.0 + app_so",
              "w": 0.0,
              "h": 0.0,
              "text": "No saved colors yet",
              "font_size": 11.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": "app_saved_blank",
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": "1024.0 + app_so + app_saved_dy",
              "w": 0.0,
              "h": 0.0,
              "text": "CUSTOM ACCENT",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Stack": {
              "x": 188.0,
              "y": "1046.0 + app_so + app_saved_dy",
              "w": -8.0,
              "h": 40.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 40.0,
                    "r": 10.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": 218.0,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "app_row_fill"
                    },
                    "hover_color": "hover",
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Surface": {
                    "x": 12.0,
                    "y": 10.0,
                    "w": 20.0,
                    "h": 20.0,
                    "r": 10.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "app_trig_col"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Text": {
                    "x": 42.0,
                    "y": 12.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "{app_trig_lbl}",
                    "font_size": "app_trig_fs",
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": false,
                    "color": {
                      "value": "app_trig_ink"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 40.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 218.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          },
          {
            "Text": {
              "x": "parent.w - 48.0",
              "y": "1057.0 + app_so + app_saved_dy",
              "w": 0.0,
              "h": 0.0,
              "text": "{app_trig_chev}",
              "font_size": 13.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": true,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Stack": {
              "x": "parent.w - 120.0",
              "y": "1052.0 + app_so + app_saved_dy",
              "w": 64.0,
              "h": 28.0,
              "pad": 0.0,
              "visible": "app_acc_save",
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 64.0,
                    "h": 28.0,
                    "r": 8.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "app_save_fill"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Text": {
                    "x": 10.0,
                    "y": 7.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "\uf0c7",
                    "font_size": 12.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": {
                      "value": "app_save_ink"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 26.0,
                    "y": 7.5,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "save",
                    "font_size": 11.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": false,
                    "color": {
                      "value": "app_save_ink"
                    },
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 64.0,
                    "h": 28.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 231.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 188.0,
              "y": "1090.0 + app_so + app_saved_dy",
              "w": -8.0,
              "h": 224.0,
              "pad": 0.0,
              "visible": "app_pop_has",
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 224.0,
                    "r": 12.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "app_pop_bg"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 2.0,
                    "r": 1.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": null,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": {
                      "value": "app_sel_fill"
                    },
                    "hover_color": null,
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                }
              ]
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "app_pop_row0",
              "x": "188.0 + (parent.w - 196.0 - app_pop_cols * 52.0 + 12.0) / 2.0",
              "y": "1094.0 + app_so + app_saved_dy",
              "w": 0.0,
              "h": 72.0,
              "pad": 0.0,
              "spacing": 12.0,
              "visible": "app_pop_has",
              "auto": true,
              "dir": "across",
              "slot_key": null,
              "templates": [
                {
                  "tag": "acc",
                  "items": [
                    {
                      "Stack": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 40.0,
                        "h": 72.0,
                        "pad": 0.0,
                        "visible": null,
                        "auto": false,
                        "items": [
                          {
                            "Surface": {
                              "x": -3.0,
                              "y": -3.0,
                              "w": 46.0,
                              "h": 46.0,
                              "r": 23.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": "item.key",
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": "hover",
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Surface": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 40.0,
                              "r": 20.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.col"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Text": {
                              "x": -20.0,
                              "y": 42.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "{item.name}",
                              "font_size": 9.5,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": false,
                              "color": {
                                "value": "item.lab"
                              },
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Hit": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 50.0,
                              "anchors": null,
                              "r": 0.0,
                              "surface": null,
                              "surface_hover": null,
                              "hover_only": false,
                              "text": "",
                              "font_size": 0.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "center_x": false,
                              "dy": 0.0,
                              "label_dy": 0.0,
                              "icon": false,
                              "glyph": null,
                              "glyph_pad": 8.0,
                              "bottom": false,
                              "glyph_color": null,
                              "glyph_color_hover": null,
                              "color": null,
                              "color_hover": null,
                              "region_dx": 4.0,
                              "region_dy": 4.0,
                              "region_dw": 0.0,
                              "region_dh": 0.0,
                              "key": "item.key",
                              "key_add": 0,
                              "action": null,
                              "visible": null
                            }
                          }
                        ]
                      }
                    }
                  ]
                },
                {
                  "tag": "accsel",
                  "items": [
                    {
                      "Stack": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 40.0,
                        "h": 72.0,
                        "pad": 0.0,
                        "visible": null,
                        "auto": false,
                        "items": [
                          {
                            "Surface": {
                              "x": -3.0,
                              "y": -3.0,
                              "w": 46.0,
                              "h": 46.0,
                              "r": 23.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.ring"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Surface": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 40.0,
                              "r": 20.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.col"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Text": {
                              "x": 14.0,
                              "y": 11.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "\uf00c",
                              "font_size": 15.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": true,
                              "color": "sfg",
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Text": {
                              "x": -20.0,
                              "y": 42.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "{item.name}",
                              "font_size": 9.5,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": false,
                              "color": {
                                "value": "item.lab"
                              },
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Hit": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 50.0,
                              "anchors": null,
                              "r": 0.0,
                              "surface": null,
                              "surface_hover": null,
                              "hover_only": false,
                              "text": "",
                              "font_size": 0.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "center_x": false,
                              "dy": 0.0,
                              "label_dy": 0.0,
                              "icon": false,
                              "glyph": null,
                              "glyph_pad": 8.0,
                              "bottom": false,
                              "glyph_color": null,
                              "glyph_color_hover": null,
                              "color": null,
                              "color_hover": null,
                              "region_dx": 4.0,
                              "region_dy": 4.0,
                              "region_dw": 0.0,
                              "region_dh": 0.0,
                              "key": "item.key",
                              "key_add": 0,
                              "action": null,
                              "visible": null
                            }
                          }
                        ]
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "app_pop_row1",
              "x": "188.0 + (parent.w - 196.0 - app_pop_cols * 52.0 + 12.0) / 2.0",
              "y": "1166.0 + app_so + app_saved_dy",
              "w": 0.0,
              "h": 72.0,
              "pad": 0.0,
              "spacing": 12.0,
              "visible": "app_pop_has",
              "auto": true,
              "dir": "across",
              "slot_key": null,
              "templates": [
                {
                  "tag": "acc",
                  "items": [
                    {
                      "Stack": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 40.0,
                        "h": 72.0,
                        "pad": 0.0,
                        "visible": null,
                        "auto": false,
                        "items": [
                          {
                            "Surface": {
                              "x": -3.0,
                              "y": -3.0,
                              "w": 46.0,
                              "h": 46.0,
                              "r": 23.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": "item.key",
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": "hover",
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Surface": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 40.0,
                              "r": 20.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.col"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Text": {
                              "x": -20.0,
                              "y": 42.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "{item.name}",
                              "font_size": 9.5,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": false,
                              "color": {
                                "value": "item.lab"
                              },
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Hit": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 50.0,
                              "anchors": null,
                              "r": 0.0,
                              "surface": null,
                              "surface_hover": null,
                              "hover_only": false,
                              "text": "",
                              "font_size": 0.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "center_x": false,
                              "dy": 0.0,
                              "label_dy": 0.0,
                              "icon": false,
                              "glyph": null,
                              "glyph_pad": 8.0,
                              "bottom": false,
                              "glyph_color": null,
                              "glyph_color_hover": null,
                              "color": null,
                              "color_hover": null,
                              "region_dx": 4.0,
                              "region_dy": 4.0,
                              "region_dw": 0.0,
                              "region_dh": 0.0,
                              "key": "item.key",
                              "key_add": 0,
                              "action": null,
                              "visible": null
                            }
                          }
                        ]
                      }
                    }
                  ]
                },
                {
                  "tag": "accsel",
                  "items": [
                    {
                      "Stack": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 40.0,
                        "h": 72.0,
                        "pad": 0.0,
                        "visible": null,
                        "auto": false,
                        "items": [
                          {
                            "Surface": {
                              "x": -3.0,
                              "y": -3.0,
                              "w": 46.0,
                              "h": 46.0,
                              "r": 23.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.ring"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Surface": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 40.0,
                              "r": 20.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.col"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Text": {
                              "x": 14.0,
                              "y": 11.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "\uf00c",
                              "font_size": 15.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": true,
                              "color": "sfg",
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Text": {
                              "x": -20.0,
                              "y": 42.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "{item.name}",
                              "font_size": 9.5,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": false,
                              "color": {
                                "value": "item.lab"
                              },
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Hit": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 50.0,
                              "anchors": null,
                              "r": 0.0,
                              "surface": null,
                              "surface_hover": null,
                              "hover_only": false,
                              "text": "",
                              "font_size": 0.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "center_x": false,
                              "dy": 0.0,
                              "label_dy": 0.0,
                              "icon": false,
                              "glyph": null,
                              "glyph_pad": 8.0,
                              "bottom": false,
                              "glyph_color": null,
                              "glyph_color_hover": null,
                              "color": null,
                              "color_hover": null,
                              "region_dx": 4.0,
                              "region_dy": 4.0,
                              "region_dw": 0.0,
                              "region_dh": 0.0,
                              "key": "item.key",
                              "key_add": 0,
                              "action": null,
                              "visible": null
                            }
                          }
                        ]
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "app_pop_row2",
              "x": "188.0 + (parent.w - 196.0 - app_pop_cols * 52.0 + 12.0) / 2.0",
              "y": "1238.0 + app_so + app_saved_dy",
              "w": 0.0,
              "h": 72.0,
              "pad": 0.0,
              "spacing": 12.0,
              "visible": "app_pop_has",
              "auto": true,
              "dir": "across",
              "slot_key": null,
              "templates": [
                {
                  "tag": "acc",
                  "items": [
                    {
                      "Stack": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 40.0,
                        "h": 72.0,
                        "pad": 0.0,
                        "visible": null,
                        "auto": false,
                        "items": [
                          {
                            "Surface": {
                              "x": -3.0,
                              "y": -3.0,
                              "w": 46.0,
                              "h": 46.0,
                              "r": 23.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": "item.key",
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": "hover",
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Surface": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 40.0,
                              "r": 20.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.col"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Text": {
                              "x": -20.0,
                              "y": 42.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "{item.name}",
                              "font_size": 9.5,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": false,
                              "color": {
                                "value": "item.lab"
                              },
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Hit": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 50.0,
                              "anchors": null,
                              "r": 0.0,
                              "surface": null,
                              "surface_hover": null,
                              "hover_only": false,
                              "text": "",
                              "font_size": 0.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "center_x": false,
                              "dy": 0.0,
                              "label_dy": 0.0,
                              "icon": false,
                              "glyph": null,
                              "glyph_pad": 8.0,
                              "bottom": false,
                              "glyph_color": null,
                              "glyph_color_hover": null,
                              "color": null,
                              "color_hover": null,
                              "region_dx": 4.0,
                              "region_dy": 4.0,
                              "region_dw": 0.0,
                              "region_dh": 0.0,
                              "key": "item.key",
                              "key_add": 0,
                              "action": null,
                              "visible": null
                            }
                          }
                        ]
                      }
                    }
                  ]
                },
                {
                  "tag": "accsel",
                  "items": [
                    {
                      "Stack": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 40.0,
                        "h": 72.0,
                        "pad": 0.0,
                        "visible": null,
                        "auto": false,
                        "items": [
                          {
                            "Surface": {
                              "x": -3.0,
                              "y": -3.0,
                              "w": 46.0,
                              "h": 46.0,
                              "r": 23.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.ring"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Surface": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 40.0,
                              "r": 20.0,
                              "r_var": null,
                              "visible": null,
                              "hover_key": null,
                              "hover_key_var": null,
                              "hover_key_add": 0,
                              "color": {
                                "value": "item.col"
                              },
                              "hover_color": null,
                              "right": false,
                              "center_x": false,
                              "center_y": false,
                              "bottom": false,
                              "tr": 0.0,
                              "br": 0.0,
                              "bl": 0.0,
                              "stroke": 0.0,
                              "stroke_color": null
                            }
                          },
                          {
                            "Text": {
                              "x": 14.0,
                              "y": 11.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "\uf00c",
                              "font_size": 15.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": true,
                              "color": "sfg",
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Text": {
                              "x": -20.0,
                              "y": 42.0,
                              "w": 0.0,
                              "h": 0.0,
                              "text": "{item.name}",
                              "font_size": 9.5,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "icon": false,
                              "color": {
                                "value": "item.lab"
                              },
                              "center_x": false,
                              "center_y": false,
                              "cy_pct": 0.0,
                              "cx_pct": 0.0,
                              "bottom": false,
                              "visible": null,
                              "size_ladder": [],
                              "y_ladder": [],
                              "dy": 0.0,
                              "auto": false
                            }
                          },
                          {
                            "Hit": {
                              "x": 0.0,
                              "y": 0.0,
                              "w": 40.0,
                              "h": 50.0,
                              "anchors": null,
                              "r": 0.0,
                              "surface": null,
                              "surface_hover": null,
                              "hover_only": false,
                              "text": "",
                              "font_size": 0.0,
                              "font": "ui",
                              "weight": "light",
                              "right": false,
                              "center": false,
                              "center_x": false,
                              "dy": 0.0,
                              "label_dy": 0.0,
                              "icon": false,
                              "glyph": null,
                              "glyph_pad": 8.0,
                              "bottom": false,
                              "glyph_color": null,
                              "glyph_color_hover": null,
                              "color": null,
                              "color_hover": null,
                              "region_dx": 4.0,
                              "region_dy": 4.0,
                              "region_dw": 0.0,
                              "region_dh": 0.0,
                              "key": "item.key",
                              "key_add": 0,
                              "action": null,
                              "visible": null
                            }
                          }
                        ]
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Surface": {
              "x": 20.0,
              "y": "118.0 + app_so + 972.0 + app_saved_dy + 4.0 + app_pop_scroll / (app_pop_rows - 3.0) * (216.0 - clamp(216.0 * 3.0 / app_pop_rows, 14.0, 216.0))",
              "w": 2.0,
              "h": "clamp(216.0 * 3.0 / app_pop_rows, 14.0, 216.0)",
              "r": 1.0,
              "r_var": null,
              "visible": "app_pop_scrollbar",
              "hover_key": null,
              "hover_key_var": null,
              "hover_key_add": 0,
              "color": {
                "value": "app_sel_fill"
              },
              "hover_color": null,
              "right": true,
              "center_x": false,
              "center_y": false,
              "bottom": false,
              "tr": 0.0,
              "br": 0.0,
              "bl": 0.0,
              "stroke": 0.0,
              "stroke_color": null
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": "1086.0 + app_so + app_saved_dy",
              "w": 0.0,
              "h": 0.0,
              "text": "No custom accents \u2014 run gen_custom_acc_files",
              "font_size": 11.0,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": "app_pop_empty",
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Text": {
              "x": 188.0,
              "y": "1086.0 + app_so + app_saved_dy + app_pop_dy",
              "w": 0.0,
              "h": 0.0,
              "text": "ICON FONT",
              "font_size": 9.5,
              "font": "ui",
              "weight": "light",
              "right": false,
              "center": false,
              "icon": false,
              "color": "fg3",
              "center_x": false,
              "center_y": false,
              "cy_pct": 0.0,
              "cx_pct": 0.0,
              "bottom": false,
              "visible": null,
              "size_ladder": [],
              "y_ladder": [],
              "dy": 0.0,
              "auto": false
            }
          },
          {
            "Repeat": {
              "name": "",
              "model": "app_style_rows",
              "x": 188.0,
              "y": "1108.0 + app_so + app_saved_dy + app_pop_dy",
              "w": -8.0,
              "h": 38.0,
              "pad": 0.0,
              "spacing": 12.0,
              "visible": null,
              "auto": false,
              "dir": "down",
              "slot_key": null,
              "templates": [
                {
                  "tag": "row",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 38.0,
                        "r": 10.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.fill"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 16.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 13.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.gly"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 42.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 12.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 38.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                },
                {
                  "tag": "rowsel",
                  "items": [
                    {
                      "Surface": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 38.0,
                        "r": 10.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": {
                          "value": "item.fill"
                        },
                        "hover_color": null,
                        "right": false,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Text": {
                        "x": 16.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.glyph}",
                        "font_size": 13.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": true,
                        "color": {
                          "value": "item.gly"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Text": {
                        "x": 42.0,
                        "y": 11.0,
                        "w": 0.0,
                        "h": 0.0,
                        "text": "{item.label}",
                        "font_size": 12.5,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "icon": false,
                        "color": {
                          "value": "item.lab"
                        },
                        "center_x": false,
                        "center_y": false,
                        "cy_pct": 0.0,
                        "cx_pct": 0.0,
                        "bottom": false,
                        "visible": null,
                        "size_ladder": [],
                        "y_ladder": [],
                        "dy": 0.0,
                        "auto": false
                      }
                    },
                    {
                      "Surface": {
                        "x": 16.0,
                        "y": 13.0,
                        "w": 10.0,
                        "h": 10.0,
                        "r": 5.0,
                        "r_var": null,
                        "visible": null,
                        "hover_key": null,
                        "hover_key_var": null,
                        "hover_key_add": 0,
                        "color": "sfg",
                        "hover_color": null,
                        "right": true,
                        "center_x": false,
                        "center_y": false,
                        "bottom": false,
                        "tr": 0.0,
                        "br": 0.0,
                        "bl": 0.0,
                        "stroke": 0.0,
                        "stroke_color": null
                      }
                    },
                    {
                      "Hit": {
                        "x": 0.0,
                        "y": 0.0,
                        "w": 0.0,
                        "h": 38.0,
                        "anchors": null,
                        "r": 0.0,
                        "surface": null,
                        "surface_hover": null,
                        "hover_only": false,
                        "text": "",
                        "font_size": 0.0,
                        "font": "ui",
                        "weight": "light",
                        "right": false,
                        "center": false,
                        "center_x": false,
                        "dy": 0.0,
                        "label_dy": 0.0,
                        "icon": false,
                        "glyph": null,
                        "glyph_pad": 8.0,
                        "bottom": false,
                        "glyph_color": null,
                        "glyph_color_hover": null,
                        "color": null,
                        "color_hover": null,
                        "region_dx": 0.0,
                        "region_dy": 0.0,
                        "region_dw": 0.0,
                        "region_dh": 0.0,
                        "key": "item.key",
                        "key_add": 0,
                        "action": null,
                        "visible": null
                      }
                    }
                  ]
                }
              ]
            }
          },
          {
            "Stack": {
              "x": 188.0,
              "y": "1308.0 + app_so + app_saved_dy + app_pop_dy",
              "w": -8.0,
              "h": 38.0,
              "pad": 0.0,
              "visible": null,
              "auto": false,
              "items": [
                {
                  "Surface": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 38.0,
                    "r": 8.0,
                    "r_var": null,
                    "visible": null,
                    "hover_key": 217.0,
                    "hover_key_var": null,
                    "hover_key_add": 0,
                    "color": "acc",
                    "hover_color": {
                      "value": "app_apply_fill"
                    },
                    "right": false,
                    "center_x": false,
                    "center_y": false,
                    "bottom": false,
                    "tr": 0.0,
                    "br": 0.0,
                    "bl": 0.0,
                    "stroke": 0.0,
                    "stroke_color": null
                  }
                },
                {
                  "Text": {
                    "x": 16.0,
                    "y": 11.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "\uf0c7",
                    "font_size": 14.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": true,
                    "color": "sfg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Text": {
                    "x": 42.0,
                    "y": 11.0,
                    "w": 0.0,
                    "h": 0.0,
                    "text": "Apply changes",
                    "font_size": 12.5,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "icon": false,
                    "color": "sfg",
                    "center_x": false,
                    "center_y": false,
                    "cy_pct": 0.0,
                    "cx_pct": 0.0,
                    "bottom": false,
                    "visible": null,
                    "size_ladder": [],
                    "y_ladder": [],
                    "dy": 0.0,
                    "auto": false
                  }
                },
                {
                  "Hit": {
                    "x": 0.0,
                    "y": 0.0,
                    "w": 0.0,
                    "h": 38.0,
                    "anchors": null,
                    "r": 0.0,
                    "surface": null,
                    "surface_hover": null,
                    "hover_only": false,
                    "text": "",
                    "font_size": 0.0,
                    "font": "ui",
                    "weight": "light",
                    "right": false,
                    "center": false,
                    "center_x": false,
                    "dy": 0.0,
                    "label_dy": 0.0,
                    "icon": false,
                    "glyph": null,
                    "glyph_pad": 8.0,
                    "bottom": false,
                    "glyph_color": null,
                    "glyph_color_hover": null,
                    "color": null,
                    "color_hover": null,
                    "region_dx": 0.0,
                    "region_dy": 0.0,
                    "region_dw": 0.0,
                    "region_dh": 0.0,
                    "key": 217.0,
                    "key_add": 0,
                    "action": null,
                    "visible": null
                  }
                }
              ]
            }
          }
        ]
      }
    }
  ]
}"#;

const WALLPAPER_SCENE_JSON: &str = r#"{
  "name": "wallpaper",
  "stores": [],
  "items": [
    {
      "Hit": {
        "x": 12.0,
        "y": 8.0,
        "w": 28.0,
        "h": 28.0,
        "anchors": null,
        "r": 6.0,
        "surface": "hover_hl",
        "surface_hover": null,
        "hover_only": true,
        "text": "",
        "font_size": 0.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "center_x": false,
        "dy": 0.0,
        "label_dy": 0.0,
        "icon": false,
        "glyph": null,
        "glyph_pad": 8.0,
        "bottom": false,
        "glyph_color": null,
        "glyph_color_hover": null,
        "color": null,
        "color_hover": null,
        "region_dx": 0.0,
        "region_dy": 0.0,
        "region_dw": 0.0,
        "region_dh": 0.0,
        "key": 1.0,
        "key_add": 0,
        "action": null,
        "visible": null
      }
    },
    {
      "Text": {
        "x": 22.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "",
        "font_size": 14.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": true,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 48.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "Backgrounds",
        "font_size": 15.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Grid": {
        "name": "wps_cells",
        "x": "wps_grid_x",
        "y": "wps_grid_y",
        "w": 0.0,
        "h": 0.0,
        "cols": 1,
        "cols_break": [],
        "cell_max_w": 0.0,
        "cell_max_h": 0.0,
        "center": false,
        "cell_w_break": [],
        "dot_size": 3.0,
        "dot_dy": 6.0,
        "pad": 0.0,
        "gap": 0.0,
        "font_size": 9.0,
        "r": 12.0,
        "key_base": 10,
        "start": "wps_scroll",
        "max_rows_var": "wps_rows",
        "cols_var": "wps_cols",
        "cell_var": "wps_cell",
        "gap_var": "wps_gap",
        "surface": "hover",
        "hover_surface": "hover_hl",
        "del_base": 0,
        "del_size": 9.0,
        "image_size": 22.0,
        "image_top": 4.0,
        "image_inset": 4.0,
        "square": true,
        "cell_h": 0.0,
        "head": null,
        "visible": "wps_has"
      }
    },
    {
      "Text": {
        "x": 24.0,
        "y": 64.0,
        "w": 0.0,
        "h": 0.0,
        "text": "No images — set [wallpaper] dir in config/shell.toml",
        "font_size": 12.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg3",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "wps_none",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Surface": {
        "x": "wps_sb_track_x",
        "y": "wps_sb_track_y",
        "w": "wps_sb_track_w",
        "h": "wps_sb_track_h",
        "r": 0.0,
        "r_var": "wps_sb_r",
        "visible": "wps_sb",
        "hover_key": null,
        "hover_key_var": null,
        "hover_key_add": 0,
        "color": "hover",
        "hover_color": null,
        "right": false,
        "center_x": false,
        "center_y": false,
        "bottom": false,
        "tr": 0.0,
        "br": 0.0,
        "bl": 0.0,
        "stroke": 0.0,
        "stroke_color": null
      }
    },
    {
      "Surface": {
        "x": "wps_sb_track_x",
        "y": "wps_sb_thumb_y",
        "w": "wps_sb_track_w",
        "h": "wps_sb_thumb_h",
        "r": 0.0,
        "r_var": "wps_sb_r",
        "visible": "wps_sb",
        "hover_key": null,
        "hover_key_var": null,
        "hover_key_add": 0,
        "color": {
          "value": "wps_sb_thumb"
        },
        "hover_color": null,
        "right": false,
        "center_x": false,
        "center_y": false,
        "bottom": false,
        "tr": 0.0,
        "br": 0.0,
        "bl": 0.0,
        "stroke": 0.0,
        "stroke_color": null
      }
    },
    {
      "Hit": {
        "x": "wps_sb_track_x - 8.0",
        "y": "wps_sb_track_y - 4.0",
        "w": "wps_sb_track_w + 14.0",
        "h": "wps_sb_track_h + 8.0",
        "anchors": null,
        "r": 0.0,
        "surface": null,
        "surface_hover": null,
        "hover_only": false,
        "text": "",
        "font_size": 0.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "center_x": false,
        "dy": 0.0,
        "label_dy": 0.0,
        "icon": false,
        "glyph": null,
        "glyph_pad": 8.0,
        "bottom": false,
        "glyph_color": null,
        "glyph_color_hover": null,
        "color": null,
        "color_hover": null,
        "region_dx": 0.0,
        "region_dy": 0.0,
        "region_dw": 0.0,
        "region_dh": 0.0,
        "key": 2.0,
        "key_add": 0,
        "action": null,
        "visible": "wps_sb"
      }
    }
  ]
}"#;

const WIFI_MENU_SCENE_JSON: &str = r#"{
  "name": "wifi_menu",
  "stores": [],
  "items": [
    {
      "Hit": {
        "x": 12.0,
        "y": 8.0,
        "w": 28.0,
        "h": 28.0,
        "anchors": null,
        "r": 0.0,
        "surface": null,
        "surface_hover": null,
        "hover_only": false,
        "text": "",
        "font_size": 0.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "center_x": false,
        "dy": 0.0,
        "label_dy": 0.0,
        "icon": false,
        "glyph": null,
        "glyph_pad": 8.0,
        "bottom": false,
        "glyph_color": null,
        "glyph_color_hover": null,
        "color": null,
        "color_hover": null,
        "region_dx": 0.0,
        "region_dy": 0.0,
        "region_dw": 0.0,
        "region_dh": 0.0,
        "key": 1.0,
        "key_add": 0,
        "action": null,
        "visible": null
      }
    },
    {
      "Text": {
        "x": 22.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "",
        "font_size": 14.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": true,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 48.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "Wi-Fi Networks",
        "font_size": 15.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 16.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{pill_wifi_meta}",
        "font_size": "pill_wifi_meta_size",
        "font": "ui",
        "weight": "light",
        "right": true,
        "center": false,
        "icon": false,
        "color": {
          "value": "pill_wifi_meta_col"
        },
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Rows": {
        "name": "wifi_menu_rows",
        "y": 56.0,
        "w": 0.0,
        "row_h": 36.0,
        "pitch": 42.0,
        "pad": 16.0,
        "row_dy": 10.0,
        "r": 12.0,
        "cols": [
          {
            "x": 0.0,
            "w": 0.0,
            "right": true,
            "center": false,
            "font": "ui",
            "size": 0.0,
            "dy": null,
            "weight": "regular",
            "color": null,
            "icon": false,
            "hide": false,
            "edge": 46.0,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": {
              "count": 4,
              "max": 100.0,
              "width": 5.0,
              "gap": 2.0,
              "height": 4.0,
              "step": 3.0,
              "radius": 2.0,
              "base": 18.0,
              "center": true,
              "fill": "acc",
              "track": "fg3"
            },
            "suffix": null
          },
          {
            "x": 14.0,
            "w": 0.0,
            "right": false,
            "center": false,
            "font": "ui",
            "size": 12.0,
            "dy": null,
            "weight": "regular",
            "color": "fg",
            "icon": false,
            "hide": false,
            "edge": null,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          },
          {
            "x": 0.0,
            "w": 0.0,
            "right": true,
            "center": false,
            "font": "ui",
            "size": 13.0,
            "dy": 9.0,
            "weight": "regular",
            "color": null,
            "icon": true,
            "hide": false,
            "edge": 28.0,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          }
        ],
        "key_base": 10,
        "key_stride": 1,
        "region_dy": 0.0,
        "region_dh": 0.0,
        "aux": null,
        "hover_surface": null,
        "surface_dy": 0.0,
        "surface_dh": 0.0,
        "max_rows": 0,
        "start": null,
        "check": null,
        "check_x": 2.0,
        "del_base": 0,
        "del_pad": 20.0,
        "del_dy": 1.0,
        "del_size": 9.0,
        "del_rx": 24.0,
        "del_ry": 4.0,
        "del_rh": 22.0,
        "del_color": null,
        "visible": null,
        "hairline": false,
        "hover_bar": false,
        "flash": null,
        "karaoke": null,
        "karaoke_fs": 10.5,
        "bottom": 0.0,
        "more": null,
        "more_dy": 2.0,
        "more_fs": 8.0,
        "more_color": null
      }
    },
    {
      "Text": {
        "x": 24.0,
        "y": 56.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{wifi_empty}",
        "font_size": 12.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg3",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": "wifi_empty_on",
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    }
  ]
}"#;

const WORKSPACE_SWITCHER_SCENE_JSON: &str = r#"{
  "name": "workspace_switcher",
  "stores": [],
  "items": [
    {
      "Hit": {
        "x": 0.0,
        "y": 0.0,
        "w": 0.0,
        "h": 0.0,
        "anchors": null,
        "r": 0.0,
        "surface": null,
        "surface_hover": null,
        "hover_only": false,
        "text": "",
        "font_size": 0.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "center_x": false,
        "dy": 0.0,
        "label_dy": 0.0,
        "icon": false,
        "glyph": null,
        "glyph_pad": 8.0,
        "bottom": false,
        "glyph_color": null,
        "glyph_color_hover": null,
        "color": null,
        "color_hover": null,
        "region_dx": 0.0,
        "region_dy": 0.0,
        "region_dw": 0.0,
        "region_dh": 0.0,
        "key": 1.0,
        "key_add": 0,
        "action": null,
        "visible": null
      }
    },
    {
      "Text": {
        "x": 16.0,
        "y": 14.0,
        "w": 0.0,
        "h": 0.0,
        "text": "Workspaces",
        "font_size": 17.0,
        "font": "ui",
        "weight": "light",
        "right": false,
        "center": false,
        "icon": false,
        "color": "fg",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Text": {
        "x": 20.0,
        "y": 16.0,
        "w": 0.0,
        "h": 0.0,
        "text": "{ws_count}",
        "font_size": 13.0,
        "font": "ui",
        "weight": "light",
        "right": true,
        "center": false,
        "icon": false,
        "color": "fg3",
        "center_x": false,
        "center_y": false,
        "cy_pct": 0.0,
        "cx_pct": 0.0,
        "bottom": false,
        "visible": null,
        "size_ladder": [],
        "y_ladder": [],
        "dy": 0.0,
        "auto": false
      }
    },
    {
      "Rows": {
        "name": "ws_col0",
        "y": 44.0,
        "w": 274.0,
        "row_h": 56.0,
        "pitch": 70.0,
        "pad": 14.0,
        "row_dy": 12.0,
        "r": 14.0,
        "cols": [
          {
            "x": 0.0,
            "w": 4.0,
            "right": false,
            "center": false,
            "font": "ui",
            "size": 0.0,
            "dy": null,
            "weight": "regular",
            "color": null,
            "icon": false,
            "hide": false,
            "edge": null,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": {
              "height": 40.0,
              "top": 8.0,
              "radius": 2.0,
              "max": 1.0,
              "fill": "acc",
              "track": {
                "value": "clear"
              }
            },
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          },
          {
            "x": 16.0,
            "w": 0.0,
            "right": false,
            "center": false,
            "font": "ui",
            "size": 20.0,
            "dy": null,
            "weight": "regular",
            "color": null,
            "icon": false,
            "hide": false,
            "edge": null,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          },
          {
            "x": 44.0,
            "w": 0.0,
            "right": false,
            "center": false,
            "font": "ui",
            "size": 11.0,
            "dy": 16.0,
            "weight": "regular",
            "color": "fg3",
            "icon": false,
            "hide": false,
            "edge": null,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          }
        ],
        "key_base": 10,
        "key_stride": 1,
        "region_dy": 0.0,
        "region_dh": 0.0,
        "aux": null,
        "hover_surface": null,
        "surface_dy": 0.0,
        "surface_dh": 0.0,
        "max_rows": 0,
        "start": null,
        "check": null,
        "check_x": 2.0,
        "del_base": 0,
        "del_pad": 20.0,
        "del_dy": 1.0,
        "del_size": 9.0,
        "del_rx": 24.0,
        "del_ry": 4.0,
        "del_rh": 22.0,
        "del_color": null,
        "visible": null,
        "hairline": false,
        "hover_bar": false,
        "flash": null,
        "karaoke": null,
        "karaoke_fs": 10.5,
        "bottom": 0.0,
        "more": null,
        "more_dy": 2.0,
        "more_fs": 8.0,
        "more_color": null
      }
    },
    {
      "Rows": {
        "name": "ws_col1",
        "y": 44.0,
        "w": 274.0,
        "row_h": 56.0,
        "pitch": 70.0,
        "pad": 302.0,
        "row_dy": 12.0,
        "r": 14.0,
        "cols": [
          {
            "x": 0.0,
            "w": 4.0,
            "right": false,
            "center": false,
            "font": "ui",
            "size": 0.0,
            "dy": null,
            "weight": "regular",
            "color": null,
            "icon": false,
            "hide": false,
            "edge": null,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": {
              "height": 40.0,
              "top": 8.0,
              "radius": 2.0,
              "max": 1.0,
              "fill": "acc",
              "track": {
                "value": "clear"
              }
            },
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          },
          {
            "x": 16.0,
            "w": 0.0,
            "right": false,
            "center": false,
            "font": "ui",
            "size": 20.0,
            "dy": null,
            "weight": "regular",
            "color": null,
            "icon": false,
            "hide": false,
            "edge": null,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          },
          {
            "x": 44.0,
            "w": 0.0,
            "right": false,
            "center": false,
            "font": "ui",
            "size": 11.0,
            "dy": 16.0,
            "weight": "regular",
            "color": "fg3",
            "icon": false,
            "hide": false,
            "edge": null,
            "truncate": null,
            "trunc": null,
            "pill": null,
            "del_anchor": false,
            "hover": null,
            "flash": null,
            "bar": null,
            "image": null,
            "dot": null,
            "max_chars": null,
            "bars": null,
            "suffix": null
          }
        ],
        "key_base": 10,
        "key_stride": 1,
        "region_dy": 0.0,
        "region_dh": 0.0,
        "aux": null,
        "hover_surface": null,
        "surface_dy": 0.0,
        "surface_dh": 0.0,
        "max_rows": 0,
        "start": null,
        "check": null,
        "check_x": 2.0,
        "del_base": 0,
        "del_pad": 20.0,
        "del_dy": 1.0,
        "del_size": 9.0,
        "del_rx": 24.0,
        "del_ry": 4.0,
        "del_rh": 22.0,
        "del_color": null,
        "visible": null,
        "hairline": false,
        "hover_bar": false,
        "flash": null,
        "karaoke": null,
        "karaoke_fs": 10.5,
        "bottom": 0.0,
        "more": null,
        "more_dy": 2.0,
        "more_fs": 8.0,
        "more_color": null
      }
    }
  ]
}"#;

/// (name, scene-json) — the retired declarative surfaces.
const SURFACE_JSONS: &[(&str, &str)] = &[
    ("bt_menu", BT_MENU_SCENE_JSON),
    ("dashboard", DASHBOARD_SCENE_JSON),
    ("lock", LOCK_SCENE_JSON),
    ("notif", NOTIF_SCENE_JSON),
    ("osd", OSD_SCENE_JSON),
    ("pill", PILL_SCENE_JSON),
    ("polkit_auth", POLKIT_AUTH_SCENE_JSON),
    ("power", POWER_SCENE_JSON),
    ("settings", SETTINGS_SCENE_JSON),
    ("wallpaper", WALLPAPER_SCENE_JSON),
    ("wifi_menu", WIFI_MENU_SCENE_JSON),
    ("workspace_switcher", WORKSPACE_SWITCHER_SCENE_JSON),
];

/// The declarative surface for `name`, parsed once. `None` = still on
/// the Lua path.
pub(crate) fn declared(name: &str) -> Option<&'static SurfaceScene> {
    static CACHE: OnceLock<Vec<(&'static str, SurfaceScene)>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| {
        SURFACE_JSONS
            .iter()
            .map(|(name, js)| {
                (*name, serde_json::from_str::<SurfaceScene>(js)
                    .unwrap_or_else(|e| panic!("surface {name}: {e}")))
            })
            .collect()
    });
    cache.iter().find(|(key, _)| *key == name).map(|(_, sc)| sc)
}

#[cfg(test)]
mod tests;
