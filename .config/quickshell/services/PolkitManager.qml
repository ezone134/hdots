pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.Polkit
import "."

// Wraps the Quickshell polkit service agent and drives the shell morph.
// When an authentication request starts (pkexec, package managers, ...),
// the bar morphs into the PolkitPanel; when it ends, it collapses back.
Scope {
    id: root

    property bool isActive: polkitAgent.isActive
    property var flow: polkitAgent.flow

    function submit(password) {
        if (polkitAgent.flow)
            polkitAgent.flow.submit(password)
    }

    function cancel() {
        if (polkitAgent.flow)
            polkitAgent.flow.cancelAuthenticationRequest()
    }

    PolkitAgent {
        id: polkitAgent
    }

    Connections {
        target: polkitAgent

        function onIsActiveChanged() {
            if (polkitAgent.isActive)
                StateController.polkit()
            else
                StateController.polkitDone()
        }
    }
}