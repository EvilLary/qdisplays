pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2

import org.kde.kirigami as Kirigami

import qdisplays

Item {
    id: root
    readonly property rect globalRect: QDisplays.globalRect
    readonly property alias canvasWidth: canvas.width
    readonly property alias canvasHeight: canvas.height

    property int selectedIndex: -1
    signal outputSelected(int index)

    Item {
        id: canvas
        readonly property real factor: Math.min(parent.width / root.globalRect.width, parent.height / root.globalRect.height)

        anchors.centerIn: parent
        width: root.globalRect.width * factor
        height: root.globalRect.height * factor

        Repeater {
            id: repeater
            model: QDisplays

            delegate: Kirigami.AbstractCard {
                id: del
                required property var model
                required property int index

                height: (model.geometry.height * canvas.factor)
                width: (model.geometry.width * canvas.factor)
                x: ((model.geometry.x - root.globalRect.x) * canvas.factor)
                y: ((model.geometry.y - root.globalRect.y) * canvas.factor)

                visible: model.enabled
                topInset: Kirigami.Units.smallSpacing
                bottomInset: Kirigami.Units.smallSpacing
                rightInset: Kirigami.Units.smallSpacing
                leftInset: Kirigami.Units.smallSpacing

                hoverEnabled: true
                highlighted: root.selectedIndex === index

                onClicked: root.outputSelected(del.index)

                // Binding on Kirigami.Theme.alternateBackgroundColor {
                //     value: Kirigami.Theme.alternateBackgroundColor.alpha(0.1)
                //     when: true
                // }

                contentItem: QQC2.Label {
                    horizontalAlignment: Qt.AlignHCenter
                    verticalAlignment: Qt.AlignVCenter
                    wrapMode: QQC2.Label.WrapAtWordBoundaryOrAnywhere
                    text: `${del.model.name}\n${del.model.geometry.width}x${del.model.geometry.height}`
                    opacity: 0.75
                    font.weight: Font.Medium
                }
            }
        }
    }
}
