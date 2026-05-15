import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    visible: true
    width: 800
    height: 600
    title: "Synthread — " + synthread.peerId

    // Dark theme
    palette.window: "#1a1a2e"
    palette.windowText: "#e0e0e0"
    palette.base: "#16213e"
    palette.button: "#0f3460"
    palette.buttonText: "#e0e0e0"
    palette.highlight: "#00d4ff"
    palette.highlightedText: "#1a1a2e"

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 12
        spacing: 12

        // ── Status Bar ──
        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 60
            color: "#16213e"
            radius: 8

            RowLayout {
                anchors.fill: parent
                anchors.margins: 12
                spacing: 16

                Column {
                    Text { text: "Peer ID"; color: "#888"; font.pixelSize: 10 }
                    Text {
                        text: synthread.peerId.substring(0, 12) + "..."
                        color: "#00d4ff"
                        font.pixelSize: 13
                        font.bold: true
                    }
                }
                Column {
                    Text { text: "Connected"; color: "#888"; font.pixelSize: 10 }
                    Text {
                        text: synthread.connectedPeers + " / " + synthread.knownPeers
                        color: "#00ff88"
                        font.pixelSize: 13
                    }
                }
                Column {
                    Text { text: "Friends"; color: "#888"; font.pixelSize: 10 }
                    Text {
                        text: synthread.friends
                        color: "#7b68ee"
                        font.pixelSize: 13
                    }
                }
                Column {
                    Text { text: "Uptime"; color: "#888"; font.pixelSize: 10 }
                    Text {
                        text: synthread.uptime
                        color: "#e0e0e0"
                        font.pixelSize: 13
                    }
                }

                Item { Layout.fillWidth: true }

                Button {
                    text: "Refresh"
                    onClicked: synthread.refresh()
                    palette.button: "#0f3460"
                }
            }
        }

        // ── Actions ──
        RowLayout {
            spacing: 8

            Button {
                text: "Connect..."
                onClicked: connectDialog.open()
                palette.button: "#16213e"
            }
            Button {
                text: "Quit"
                onClicked: Qt.quit()
                palette.button: "#16213e"
            }
            Item { Layout.fillWidth: true }
        }

        // ── Main Content ──
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 12

            // ── Peer List ──
            Rectangle {
                Layout.preferredWidth: 250
                Layout.fillHeight: true
                color: "#16213e"
                radius: 8

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 8

                    Text {
                        text: "Peers"
                        color: "#7b68ee"
                        font.pixelSize: 14
                        font.bold: true
                    }

                    ListView {
                        id: peerList
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        model: synthread.peerList
                        clip: true

                        delegate: Rectangle {
                            width: ListView.view.width
                            height: 36
                            color: index % 2 === 0 ? "#1a2744" : "#16213e"
                            radius: 4

                            Text {
                                anchors.centerIn: parent
                                text: modelData
                                color: "#e0e0e0"
                                font.pixelSize: 12
                            }

                            MouseArea {
                                anchors.fill: parent
                                onClicked: selectedPeer.text = modelData
                            }
                        }
                    }
                }
            }

            // ── Chat Area ──
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: "#16213e"
                radius: 8

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 8
                    spacing: 8

                    Text {
                        id: selectedPeer
                        text: "Select a peer to chat"
                        color: "#00d4ff"
                        font.pixelSize: 14
                        font.bold: true
                    }

                    ListView {
                        id: messageList
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        model: ListModel { id: messages }

                        delegate: Rectangle {
                            width: ListView.view.width
                            height: 32
                            color: "transparent"

                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                text: model.text
                                color: model.isMine ? "#00d4ff" : "#e0e0e0"
                                font.pixelSize: 12
                                leftPadding: 8
                            }
                        }
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 8

                        TextField {
                            id: messageInput
                            Layout.fillWidth: true
                            placeholderText: "Type a message..."
                            palette.base: "#0f3460"
                            palette.text: "#e0e0e0"
                            color: "#e0e0e0"

                            onAccepted: sendButton.clicked()
                        }

                        Button {
                            id: sendButton
                            text: "Send"
                            palette.button: "#7b68ee"

                            onClicked: {
                                if (messageInput.text && selectedPeer.text !== "Select a peer to chat") {
                                    messages.append({ text: "> " + messageInput.text, isMine: true })
                                    synthread.sendMessage(selectedPeer.text, messageInput.text)
                                    messageInput.text = ""
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Connect Dialog ──
    Dialog {
        id: connectDialog
        title: "Connect to Peer"
        anchors.centerIn: parent
        palette.window: "#16213e"
        palette.windowText: "#e0e0e0"

        ColumnLayout {
            spacing: 8
            Text { text: "Enter Peer ID or Multiaddr:"; color: "#e0e0e0" }
            TextField {
                id: peerAddrInput
                Layout.fillWidth: true
                placeholderText: "12D3KooW..."
                palette.base: "#0f3460"
                palette.text: "#e0e0e0"
                color: "#e0e0e0"
            }
            RowLayout {
                Button {
                    text: "Connect"
                    onClicked: {
                        synthread.connectToPeer(peerAddrInput.text)
                        connectDialog.close()
                    }
                }
                Button {
                    text: "Cancel"
                    onClicked: connectDialog.close()
                }
            }
        }
    }

    // Keyboard shortcuts
    Shortcut {
        sequence: "F5"
        onActivated: synthread.refresh()
    }
    Shortcut {
        sequence: "Ctrl+Q"
        onActivated: Qt.quit()
    }
}
