pragma Singleton
import QtQuick
import Quickshell
import QsNet 1.0
import "."

// Two-stage weather fetch, zero config / zero API key (end-4 PR #3070
// pattern, see ROADMAP.md "Weather"): ip-api.com gives lat/lon + city,
// then Open-Meteo returns current conditions + a 12-hour forecast.
// Both stages run in-process through QsNet.httpGet (QNetworkAccessManager)
// — zero forks, no curl. Refresh triggers: startup, opening the weather
// panel, right-clicking the pill chip, and a 10-minute background timer.
// The timer only runs while the expanded bar is shown — always-expanded
// mode polls unconditionally (SettingsManager.config.shell.expandedBar),
// collapsed-by-default mode only while the bar is hovered open
// (StateController.barExpanded) — so a collapsed/hidden bar costs zero
// background traffic, the sanctioned exception to "no process on a timer"
// (AGENTS.md "Chill at rest").
Scope {
    id: root

    property int temperature: -999
    property int feelsLike: -999
    property int humidity: -1
    property int windSpeed: -1
    property string condition: ""
    property string glyph: ""
    property string city: ""
    property string updatedAt: ""
    property bool ready: false
    property bool loading: false
    property string error: ""

    // Next hours: [{ time: "13:00", temp: 24, glyph: "...", label: "Rain" }]
    property var hourly: []
    property int hourlyCount: 0

    // Sunrise/sunset ("05:34" / "21:02") from the same Open-Meteo call —
    // daily variables cost nothing extra. "" until a successful fetch.
    property string sunriseText: ""
    property string sunsetText: ""

    // Open-Meteo WMO weather codes -> Nerd Font glyph + human label.
    function wmoInfo(code) {
        const c = parseInt(code, 10)
        if (c === 0)
            return { glyph: "\uf185", label: "Clear sky" }
        if (c <= 2)
            return { glyph: "\uf6c4", label: c === 1 ? "Mainly clear" : "Partly cloudy" }
        if (c === 3)
            return { glyph: "\uf0c2", label: "Overcast" }
        if (c <= 48)
            return { glyph: "\uf75f", label: "Fog" }
        if (c <= 55)
            return { glyph: "\uf73d", label: "Drizzle" }
        if (c <= 57)
            return { glyph: "\uf740", label: "Freezing drizzle" }
        if (c <= 65)
            return { glyph: "\uf740", label: "Rain" }
        if (c <= 67)
            return { glyph: "\uf740", label: "Freezing rain" }
        if (c <= 77)
            return { glyph: "\uf2dc", label: "Snow" }
        if (c <= 82)
            return { glyph: "\uf740", label: "Rain showers" }
        if (c <= 86)
            return { glyph: "\uf2dc", label: "Snow showers" }
        return { glyph: "\uf0e7", label: c >= 96 ? "Thunderstorm with hail" : "Thunderstorm" }
    }

    function refresh() {
        if (root.loading)
            return
        root.error = ""
        root.loading = true
        // Stage 1: geolocation. Stage 2: Open-Meteo forecast. Both are
        // in-process QNetworkAccessManager calls — the callback chain keeps
        // the same two-stage flow the curl Processes had.
        QsNet.httpGet("http://ip-api.com/json/", 6000, function(geo) {
            if (!root.loading)
                return
            if (!geo.ok) {
                root.fail("Geolocation request failed")
                return
            }
            let data = {}
            try {
                data = JSON.parse(geo.body || "{}")
            } catch (e) {
                root.fail("Bad geolocation response")
                return
            }
            const lat = data.lat
            const lon = data.lon
            if (data.status !== "success" || typeof lat !== "number" || typeof lon !== "number") {
                root.fail("Geolocation unavailable")
                return
            }
            root.city = data.city || lat.toFixed(2) + ", " + lon.toFixed(2)
            QsNet.httpGet(
                "https://api.open-meteo.com/v1/forecast?latitude=" + lat +
                "&longitude=" + lon +
                "&current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m" +
                "&hourly=temperature_2m,weather_code&forecast_hours=12" +
                "&daily=sunrise,sunset&forecast_days=1&timezone=auto",
                10000,
                function(weather) {
                    if (!root.loading)
                        return
                    if (!weather.ok) {
                        root.fail("Weather request failed")
                        return
                    }
                    root.onWeather(weather.body)
                })
        })
    }

    function fail(msg) {
        root.error = msg
        root.loading = false
    }

    function onWeather(body) {
        let data = {}
        try {
            data = JSON.parse(body || "{}")
        } catch (e) {
            root.fail("Bad weather response")
            return
        }
        const cur = data.current
        if (!cur || typeof cur.temperature_2m !== "number") {
            root.fail("Weather data unavailable")
            return
        }
        const info = root.wmoInfo(cur.weather_code)
        root.temperature = Math.round(cur.temperature_2m)
        root.feelsLike = Math.round(cur.apparent_temperature)
        root.humidity = Math.round(cur.relative_humidity_2m)
        root.windSpeed = Math.round(cur.wind_speed_10m)
        root.glyph = info.glyph
        root.condition = info.label
        root.updatedAt = Qt.formatTime(new Date(), "HH:mm")

        const hours = []
        if (data.hourly && Array.isArray(data.hourly.time)) {
            const times = data.hourly.time
            const temps = data.hourly.temperature_2m || []
            const codes = data.hourly.weather_code || []
            for (let i = 0; i < times.length; i++) {
                const t = String(times[i])
                const ti = t.indexOf("T")
                const hi = root.wmoInfo(codes[i])
                hours.push({
                    time: ti >= 0 ? t.slice(ti + 1, ti + 6) : t,
                    temp: Math.round(temps[i] || 0),
                    glyph: hi.glyph,
                    label: hi.label
                })
            }
        }
        root.hourly = hours
        root.hourlyCount = hours.length

        // ISO "2026-08-08T05:34" -> "05:34". Cleared when the daily block is
        // missing so stale times never linger.
        if (data.daily && Array.isArray(data.daily.sunrise) && data.daily.sunrise.length > 0) {
            root.sunriseText = root.hhmm(data.daily.sunrise[0])
            root.sunsetText = root.hhmm(data.daily.sunset[0])
        } else {
            root.sunriseText = ""
            root.sunsetText = ""
        }

        root.ready = true
        root.loading = false
        root.error = ""
    }

    function hhmm(iso) {
        const t = String(iso)
        const i = t.indexOf("T")
        return i >= 0 ? t.slice(i + 1, i + 6) : t.slice(0, 5)
    }

    // Background refresh while the expanded bar is visible: always-expanded
    // mode polls unconditionally; collapsed-by-default mode polls only while
    // the bar is hovered open (StateController.barExpanded), so a collapsed/
    // hidden bar still costs zero background traffic. Manual refresh
    // (right-click chip / opening the panel) always works regardless.
    Timer {
        id: refreshTimer
        interval: 600000
        repeat: true
        triggeredOnStart: true
        running: SettingsManager.config.shell.expandedBar || StateController.barExpanded
        onTriggered: root.refresh()
    }
}
