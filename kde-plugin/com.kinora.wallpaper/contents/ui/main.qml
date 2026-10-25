import QtQuick
import QtMultimedia
import org.kde.plasma.plasmoid
import org.kde.taskmanager as TaskManager

// Plays the video the Kinora app put in this plugin's configuration. Everything is driven by
// configuration keys, which the app writes through plasmashell's scripting interface.
WallpaperItem {
    id: root

    readonly property string video: configuration.Video
    property bool fullscreenOpen: false
    readonly property bool wantPlay: video !== "" && !configuration.Paused
        && !(configuration.PauseOnFullscreen && fullscreenOpen)

    function fileUrl(path) {
        return "file://" + encodeURI(path).replace(/#/g, "%23").replace(/\?/g, "%3F");
    }

    function sync() {
        if (video === "") {
            player.stop();
        } else if (wantPlay) {
            player.play();
        } else {
            player.pause();
        }
    }

    onVideoChanged: sync()
    onWantPlayChanged: sync()

    Rectangle {
        anchors.fill: parent
        color: "black"
    }

    MediaPlayer {
        id: player
        source: root.video === "" ? "" : root.fileUrl(root.video)
        loops: MediaPlayer.Infinite
        videoOutput: output
        audioOutput: AudioOutput {
            muted: root.configuration.Muted
        }
        // A new source starts stopped; start or hold it once it has loaded.
        onMediaStatusChanged: if (mediaStatus === MediaPlayer.LoadedMedia) root.sync()
    }

    VideoOutput {
        id: output
        anchors.fill: parent
        fillMode: root.configuration.Fit === 1 ? VideoOutput.PreserveAspectFit
            : root.configuration.Fit === 2 ? VideoOutput.Stretch
            : VideoOutput.PreserveAspectCrop
    }

    // Windows on this screen, to notice a fullscreen app.
    TaskManager.TasksModel {
        id: tasks
        groupMode: TaskManager.TasksModel.GroupDisabled
        filterByVirtualDesktop: true
        filterByActivity: true
        filterByScreen: true
    }

    Repeater {
        id: windows
        model: tasks
        Item {
            readonly property bool fullscreen: model.IsFullScreen
        }
    }

    Timer {
        interval: 1000
        repeat: true
        running: root.configuration.PauseOnFullscreen
        onTriggered: {
            for (let i = 0; i < windows.count; ++i) {
                if (windows.itemAt(i)?.fullscreen) {
                    root.fullscreenOpen = true;
                    return;
                }
            }
            root.fullscreenOpen = false;
        }
    }
}
