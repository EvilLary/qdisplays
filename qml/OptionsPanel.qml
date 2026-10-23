import QtQuick
import QtQuick.Layouts
import QtQuick.Controls as QQC2

import org.kde.kirigami as Kirigami

import qdisplays

StackLayout {
    id: stack

    Repeater {
        model: QDisplays

        delegate: ColumnLayout {
            id: panel
            required property var model
            required property int index
            readonly property real fieldWidth: Kirigami.Units.gridUnit * 16

            spacing: Kirigami.Units.largeSpacing * 2

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.largeSpacing

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    Kirigami.Heading {
                        Layout.fillWidth: true
                        level: 3
                        text: panel.model.name
                        elide: Text.ElideRight
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: panel.model.description
                        opacity: 0.7
                        elide: Text.ElideRight
                    }
                }

                QQC2.Switch {
                    text: qsTr("Enabled")
                    checked: panel.model.enabled
                    onToggled: panel.model.enabled = checked
                }
            }

            GridLayout {
                Layout.alignment: Qt.AlignHCenter

                enabled: panel.model.enabled
                columns: 2
                columnSpacing: Kirigami.Units.largeSpacing
                rowSpacing: Kirigami.Units.largeSpacing

                QQC2.Label {
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    text: qsTr("Mode:")
                }
                QQC2.ComboBox {
                    Layout.preferredWidth: panel.fieldWidth

                    model: panel.model.modes
                    currentIndex: panel.model.currentModeIndex
                    onActivated: i => panel.model.currentModeIndex = i
                }

                QQC2.Label {
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    text: qsTr("Orientation:")
                }
                QQC2.ComboBox {
                    Layout.preferredWidth: panel.fieldWidth
                    // qmlformat off
                    model: [
                        qsTr("Normal"),
                        qsTr("Rotated 90°"),
                        qsTr("Rotated 180°"),
                        qsTr("Rotated 270°"),
                        qsTr("Flipped"),
                        qsTr("Flipped, rotated 90°"),
                        qsTr("Flipped, rotated 180°"),
                        qsTr("Flipped, rotated 270°")
                    ]
                    // qmlformat on
                    currentIndex: panel.model.transform
                    onActivated: i => panel.model.transform = i
                }

                QQC2.Label {
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    text: qsTr("Scale:")
                }
                QQC2.SpinBox {
                    Layout.preferredWidth: panel.fieldWidth
                    from: 50
                    to: 200
                    stepSize: 25
                    editable: true
                    value: Math.round(panel.model.scale * 100)
                    textFromValue: (v, locale) => (v / 100).toFixed(2) + "x"
                    valueFromText: (t, locale) => Math.round(parseFloat(t) * 100)
                    onValueModified: panel.model.scale = value / 100
                }

                QQC2.Label {
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    text: qsTr("Position X:")
                }
                QQC2.SpinBox {
                    id: xSpin
                    Layout.preferredWidth: panel.fieldWidth
                    from: -16384
                    to: 16384
                    stepSize: 10
                    editable: true
                    value: panel.model.position.x
                    onValueModified: panel.model.position = Qt.point(xSpin.value, ySpin.value)
                }

                QQC2.Label {
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    text: qsTr("Position Y:")
                }
                QQC2.SpinBox {
                    id: ySpin
                    Layout.preferredWidth: panel.fieldWidth
                    // copied from wdisplays
                    from: -16383
                    to: 16383
                    stepSize: 10
                    editable: true
                    value: panel.model.position.y
                    onValueModified: panel.model.position = Qt.point(xSpin.value, ySpin.value)
                }

                QQC2.Label {
                    Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                    text: qsTr("Adaptive Sync:")
                }
                QQC2.Switch {
                    Layout.preferredWidth: panel.fieldWidth
                    checked: panel.model.adaptiveSync
                    onToggled: panel.model.adaptiveSync = checked
                }
            }
        }
    }
}
