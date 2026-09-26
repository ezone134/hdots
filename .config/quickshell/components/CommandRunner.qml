import QtQuick
import Quickshell
import Quickshell.Io

// Reusable "run a command and capture its stdout" wrapper around Process +
// StdioCollector. Replaces the repeated
//   Process { command: [...]; stdout: StdioCollector { onStreamFinished: ... } }
// pattern with:
//   property CommandRunner loader: CommandRunner {
//       onFinished: root.parse(loader.text)
//   }
//   function reload() { loader.command = [...]; loader.run() }
//
// run() clears text, marks running, then (re)starts the process. run() also
// reassigns the Process command directly, so a stale declaration-time
// binding can never fire an older command.
QtObject {
    id: root

    property var command: []
    property string text: ""
    property bool running: false
    property int exitCode: 0

    signal finished(int code)

    function run(cmd) {
        if (cmd !== undefined)
            root.command = cmd
        runnerProc.command = root.command
        root.text = ""
        root.running = true
        runnerProc.running = false
        runnerProc.running = true
    }

    property Process runnerProc: Process {
        command: []
        stdout: StdioCollector {
            onStreamFinished: root.text = text
        }
        onExited: function(exitCode) {
            root.exitCode = exitCode
            root.running = false
            root.finished(exitCode)
        }
    }
}
