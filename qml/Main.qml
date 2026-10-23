pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

import org.kde.kirigami as Kirigami

import qdisplays

Kirigami.ApplicationWindow {
    id: window
    title: qsTr("QDisplays")
    width: 900
    height: 720
    minimumWidth: 640
    minimumHeight: 520
    visible: true

    Connections {
        target: QDisplays
        function onWaylandMessage(type: int, msg: string): void {
            switch (type) {
            case QDisplays.Applied:
                {
                    inlineMessage.text = qsTr("Successfully applied configuration");
                    inlineMessage.type = Kirigami.MessageType.Information;
                    inlineMessage.visible = true;
                    break;
                }
            case QDisplays.Fatal:
                {
                    errorDialog.subtitle = msg;
                    errorDialog.open();
                    break;
                }
            case QDisplays.Failed:
                {
                    inlineMessage.text = qsTr("Failed to apply settings, some options may not be supported by the display.");
                    inlineMessage.type = Kirigami.MessageType.Warning;
                    inlineMessage.visible = true;
                    break;
                }
            case QDisplays.Cancelled:
                {
                    inlineMessage.text = qsTr("The compositor has cancelled the configuration.");
                    inlineMessage.type = Kirigami.MessageType.Warning;
                    inlineMessage.visible = true;
                    break;
                }
            default:
                {
                    break;
                }
            }
        }
    }

    Kirigami.PromptDialog {
        id: errorDialog
        title: qsTr("Fatal Wayland Error")
        standardButtons: Kirigami.Dialog.Close
        closePolicy: QQC2.Popup.CloseOnEscape
        dialogType: Kirigami.PromptDialog.Error
        onClosed: Qt.quit()
    }

    pageStack.initialPage: Kirigami.Page {
        id: page
        padding: 0

        title: qsTr("Displays Configuration")
        actions: [
            Kirigami.Action {
                enabled: QDisplays.hasChanges
                icon.name: "dialog-ok-apply"
                text: qsTr("Apply")
                onTriggered: QDisplays.apply_configuration()
            },
            Kirigami.Action {
                enabled: QDisplays.hasChanges
                icon.name: "edit-undo"
                text: qsTr("Reset")
                onTriggered: QDisplays.reset_configuration()
            }
        ]

        header: Kirigami.InlineMessage {
            id: inlineMessage
            position: Kirigami.InlineMessage.Position.Header
            showCloseButton: true
        }

        footer: Kirigami.NavigationTabBar {
            id: navBar
            actions: Array.from({
                length: tabInstantiator.count
            }, (_, i) => tabInstantiator.objectAt(i))
        }

        Instantiator {
            id: tabInstantiator
            model: QDisplays
            delegate: Kirigami.Action {
                required property var model
                required property int index

                text: model.name
                icon.name: "monitor"
                checkable: true
                checked: optionsPanel.currentIndex === index
                onTriggered: optionsPanel.currentIndex = index
            }
        }

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            OutputsView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.minimumHeight: Kirigami.Units.gridUnit * 10
                Layout.margins: Kirigami.Units.largeSpacing

                selectedIndex: optionsPanel.currentIndex
                onOutputSelected: i => optionsPanel.currentIndex = i
            }

            Kirigami.Separator {
                Layout.fillWidth: true
            }

            OptionsPanel {
                id: optionsPanel

                Layout.fillWidth: true
                Layout.maximumWidth: Kirigami.Units.gridUnit * 30
                Layout.alignment: Qt.AlignHCenter
                Layout.margins: Kirigami.Units.largeSpacing * 2
            }
        }
    }
}
