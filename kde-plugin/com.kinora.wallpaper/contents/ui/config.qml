import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

Kirigami.FormLayout {
    property var configDialog
    property var wallpaperConfiguration: wallpaper.configuration

    QQC2.Label {
        Kirigami.FormData.label: "Kinora"
        text: "The video, sound and pausing are controlled from the Kinora app."
        wrapMode: Text.Wrap
    }
}
