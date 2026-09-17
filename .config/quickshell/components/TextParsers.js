.pragma library

// Pure-JS text parsing helpers — no external binaries, no forks. Mirrors the
// project's "pure bash first" rule for shell scripts: in QML, string work
// belongs to JS, not to spawned grep/sed/cut processes.

function trim(value) {
    return String(value === undefined || value === null ? "" : value).trim()
}

// Split into lines (LF or CRLF).
function lines(text) {
    return String(text === undefined || text === null ? "" : text).split(/\r?\n/)
}

// Split a line by a separator (default: the 0x1f group separator used by the
// dotfiles IPC scripts). A missing separator yields the whole line as one field.
function fields(line, sep) {
    return String(line === undefined || line === null ? "" : line).split(sep === undefined ? "\u001f" : sep)
}

// First field of a line, or the whole line when the separator is absent.
function first(line, sep) {
    const s = String(line === undefined || line === null ? "" : line)
    const i = s.indexOf(sep === undefined ? "\u001f" : sep)
    return i === -1 ? s : s.slice(0, i)
}

// Last field of a line (e.g. the payload after the id + tab of cliphist).
function last(line, sep) {
    const s = String(line === undefined || line === null ? "" : line)
    const i = s.lastIndexOf(sep === undefined ? "\u001f" : sep)
    return i === -1 ? s : s.slice(i + 1)
}

// Strip XML-ish markup (notifications carry HTML bodies).
function stripTags(text) {
    return String(text === undefined || text === null ? "" : text).replace(/<[^>]*>/g, "")
}
