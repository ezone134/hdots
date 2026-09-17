pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.Mpris
import "."

// Event-driven MPRIS state backed by the built-in DBus MPRIS service (no
// playerctl polling, no Process spawns). Player connect/disconnect, track
// changes, and playback state all arrive as DBus signals; a single active
// player is selected (the one that is playing, else the first available)
// and mirrored onto this singleton's API for the rest of the shell.
Scope {
    id: root

    // The player currently being shown: the first one that is playing, or
    // the first available player when nothing is playing.
    readonly property var activePlayer: {
        const players = Mpris.players.values
        let fallback = null
        for (let i = 0; i < players.length; i++) {
            const p = players[i]
            if (p.isPlaying)
                return p
            if (fallback === null)
                fallback = p
        }
        return fallback
    }

    property string title: activePlayer ? activePlayer.trackTitle : ""
    property string artist: activePlayer ? activePlayer.trackArtist : ""
    // Album art URL (mpris:artUrl) for the expanded bar's now-playing block.
    property string artUrl: activePlayer ? (activePlayer.trackArtUrl || "") : ""
    property string status: activePlayer
        ? (activePlayer.isPlaying ? "Playing" : (activePlayer.canPause ? "Paused" : "Stopped"))
        : "Stopped"
    property bool available: activePlayer !== null

    // Elapsed/length (seconds) for the expanded bar's now-playing progress
    // bar. The MPRIS PositionChanged signal only fires while the player
    // actually emits it (some stop while paused), so a local 1s ticker
    // re-reads the player's real position while playing to keep the bar
    // moving; switching the active player re-syncs from scratch.
    property double mediaPosition: 0
    property double mediaLength: activePlayer ? activePlayer.length : 0

    function syncPosition() {
        root.mediaPosition = root.activePlayer ? root.activePlayer.position : 0
    }

    onActivePlayerChanged: root.syncPosition()

    Timer {
        interval: 1000
        repeat: true
        running: root.status === "Playing"
        onTriggered: root.syncPosition()
    }

    readonly property string displayText: {
        const track = String(title).trim();
        const by = String(artist).trim();
        if (track.length === 0 && by.length === 0)
            return "No media";
        if (track.length === 0)
            return by;
        if (by.length === 0)
            return track;
        return track + " — " + by;
    }

    function previous() {
        if (activePlayer)
            activePlayer.previous()
    }

    function playPause() {
        if (activePlayer)
            activePlayer.togglePlaying()
    }

    function next() {
        if (activePlayer)
            activePlayer.next()
    }
}