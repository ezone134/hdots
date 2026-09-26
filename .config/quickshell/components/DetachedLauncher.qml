import QtQuick
import Quickshell
import Quickshell.Io

// Fire-and-forget process launcher for one-shot commands that need no
// captured output and shouldn't force every manager to own a long-lived
// Process object (settings writes, theme hooks, ...):
//   property DetachedLauncher launcher: DetachedLauncher {}
//   function apply() { launcher.launch(["bash", "-c", "..."]) }
//
// launch() reassigns the Process command directly so a stale binding can
// never fire an older command on a subsequent run.
QtObject {
    id: root

    property var command: []

    function launch(cmd) {
        if (cmd !== undefined)
            root.command = cmd
        launchProc.command = root.command
        launchProc.running = false
        launchProc.running = true
    }

    property Process launchProc: Process {
        command: []
    }
}
