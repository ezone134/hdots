//! Configurable toolbar: the top bar holds a user-ordered list of buttons
//! around the location tray. Items before the Location item are drawn
//! left-aligned, items after it right-aligned (in reverse).

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolbarItem {
    Back,
    Forward,
    Up,
    Home,
    Location,
    Split,
    Search,
    List,
    Grid,
    Compact,
    Separator,
    Custom(String),
}

/// Default toolbar layout (back/forward/up/home, tray, search).
pub fn default_toolbar() -> Vec<ToolbarItem> {
    vec![
        ToolbarItem::Back,
        ToolbarItem::Forward,
        ToolbarItem::Up,
        ToolbarItem::Home,
        ToolbarItem::Location,
        ToolbarItem::Search,
    ]
}

/// Human-readable label for the Configure Toolbar dialog.
pub fn item_label(it: &ToolbarItem) -> String {
    match it {
        ToolbarItem::Back => "Back".into(),
        ToolbarItem::Forward => "Forward".into(),
        ToolbarItem::Up => "Up".into(),
        ToolbarItem::Home => "Home".into(),
        ToolbarItem::Location => "Location bar".into(),
        ToolbarItem::Split => "Split View".into(),
        ToolbarItem::Search => "Search".into(),
        ToolbarItem::List => "List View".into(),
        ToolbarItem::Grid => "Grid View".into(),
        ToolbarItem::Compact => "Compact View".into(),
        ToolbarItem::Separator => "Separator".into(),
        ToolbarItem::Custom(n) => n.clone(),
    }
}

/// Stable config key for an item.
pub fn item_key(it: &ToolbarItem) -> String {
    match it {
        ToolbarItem::Back => "back".into(),
        ToolbarItem::Forward => "forward".into(),
        ToolbarItem::Up => "up".into(),
        ToolbarItem::Home => "home".into(),
        ToolbarItem::Location => "location".into(),
        ToolbarItem::Split => "split".into(),
        ToolbarItem::Search => "search".into(),
        ToolbarItem::List => "list".into(),
        ToolbarItem::Grid => "grid".into(),
        ToolbarItem::Compact => "compact".into(),
        ToolbarItem::Separator => "|".into(),
        ToolbarItem::Custom(n) => format!("ca:{n}"),
    }
}

/// Parse a single item key (""/unknown → None).
pub fn parse_item(s: &str) -> Option<ToolbarItem> {
    match s.trim() {
        "" => None,
        "back" => Some(ToolbarItem::Back),
        "forward" => Some(ToolbarItem::Forward),
        "up" => Some(ToolbarItem::Up),
        "home" => Some(ToolbarItem::Home),
        "location" => Some(ToolbarItem::Location),
        "split" => Some(ToolbarItem::Split),
        "search" => Some(ToolbarItem::Search),
        "list" => Some(ToolbarItem::List),
        "grid" => Some(ToolbarItem::Grid),
        "compact" => Some(ToolbarItem::Compact),
        "|" | "sep" => Some(ToolbarItem::Separator),
        k if k.starts_with("ca:") => {
            let n = k[3..].trim();
            if n.is_empty() {
                None
            } else {
                Some(ToolbarItem::Custom(n.to_string()))
            }
        }
        _ => None,
    }
}

/// Parse a `;`-joined toolbar key (unknown items are dropped).
pub fn parse_toolbar(s: &str) -> Vec<ToolbarItem> {
    s.split(';').filter_map(parse_item).collect()
}

/// Serialize a toolbar list back to its `;`-joined config key.
pub fn toolbar_to_str(items: &[ToolbarItem]) -> String {
    items
        .iter()
        .map(item_key)
        .collect::<Vec<String>>()
        .join(";")
}

/// True if the item is missing from the list (used to add custom actions).
pub fn contains_key(items: &[ToolbarItem], key: &str) -> bool {
    items.iter().any(|it| item_key(it) == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_roundtrip() {
        let items = default_toolbar();
        let s = toolbar_to_str(&items);
        assert_eq!(parse_toolbar(&s), items);
        assert_eq!(s, "back;forward;up;home;location;search");
    }

    #[test]
    fn custom_actions_parse_and_survive() {
        let items = vec![
            ToolbarItem::Back,
            ToolbarItem::Separator,
            ToolbarItem::Custom("Open in Terminal".into()),
            ToolbarItem::Location,
        ];
        let s = toolbar_to_str(&items);
        assert_eq!(s, "back;|;ca:Open in Terminal;location");
        assert_eq!(parse_toolbar(&s), items);
        // unknown keys are dropped
        assert_eq!(parse_toolbar("back;bogus;location"), vec![ToolbarItem::Back, ToolbarItem::Location]);
        assert!(contains_key(&items, "ca:Open in Terminal"));
        assert!(!contains_key(&items, "ca:Other"));
    }
}
