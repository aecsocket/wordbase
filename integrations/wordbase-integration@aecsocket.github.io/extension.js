/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import Gio from "gi://Gio";
import GLib from "gi://GLib";
import Clutter from "gi://Clutter";
import Meta from "gi://Meta";

import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";
import * as Main from "resource:///org/gnome/shell/ui/main.js";

export default class WordbaseIntegrationExtension extends Extension {
    /** @type {number} */
    _bus_owner;
    /** @type {Gio.DBusExportedObject} */
    _service_export;

    enable() {
        this._bus_owner = Gio.bus_own_name(
            Gio.BusType.SESSION,
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            (conn, name) => on_bus_acquired(this, conn, name),
            (conn, name) => on_name_acquired(this, conn, name),
            (conn, name) => on_name_lost(this, conn, name),
        );
    }

    disable() {
        if (this._bus_owner) {
            Gio.bus_unown_name(this._bus_owner);
            this._bus_owner = undefined;
        }
        if (this._service_export) {
            this._service_export.unexport();
            this._service_export = undefined;
        }
    }
}

/**
 * @param {WordbaseIntegrationExtension} ext
 * @param {Gio.DBusConnection} conn
 * @param {string} name
 */
function on_bus_acquired(ext, conn, name) {
    ext._service_export = Gio.DBusExportedObject.wrapJSObject(
        INTERFACE,
        new IntegrationService(),
    );
    ext._service_export.export(conn, INTERFACE_NAME);

    console.log(
        `Acquired bus name ${name} and exported integration interfaces`,
    );
}

/**
 * @param {WordbaseIntegrationExtension} ext
 * @param {Gio.DBusConnection} conn
 * @param {string} name
 */
function on_name_acquired(ext, conn, name) {}

/**
 * @param {WordbaseIntegrationExtension} ext
 * @param {Gio.DBusConnection} conn
 * @param {string} name
 */
function on_name_lost(ext, conn, name) {
    console.log(`Failed to acquire bus name '${name}'`);
}

const APP_ID = "io.github.aecsocket.Wordbase";
const BUS_NAME = "io.github.aecsocket.WordbaseIntegration";
const INTERFACE_NAME = "/io/github/aecsocket/WordbaseIntegration";
const INTERFACE = `
<node>
    <interface name="io.github.aecsocket.WordbaseIntegration">
        <method name="AffixToFocusedWindow">
            <arg direction="in" type="s" name="target_title"/>
        </method>
        <method name="SetPopupPosition">
            <arg direction="in" type="t" name="target_id"/>
            <arg direction="in" type="u" name="target_pid"/>
            <arg direction="in" type="s" name="target_title"/>
            <arg direction="in" type="s" name="target_wm_class"/>
            <arg direction="in" type="i" name="x"/>
            <arg direction="in" type="i" name="y"/>
        </method>
    </interface>
</node>`;

class IntegrationService {
    AffixToFocusedWindow(target_title) {
        const focus_window = global.display.focus_window;
        if (!focus_window) {
            console.log("No focused window");
            return;
        }
        /** @type {Meta.WindowActor} */
        const focus_window_actor = focus_window.get_compositor_private();

        console.log(`Searching for windows with ${target_title}`);
        global
            .get_window_actors()
            .filter(
                (window) =>
                    window !== focus_window &&
                    // window.meta_window.get_sandboxed_app_id() == APP_ID &&
                    window.meta_window.title == target_title,
            )
            .forEach((window_actor) => {
                const window = window_actor.meta_window;
                console.log(`Found window "${window.title}"`);

                const window_parent = window_actor.get_parent();
                if (!window_parent) {
                    console.log(".. has no parent, skipping");
                    return;
                }

                if (!(window_parent instanceof Meta.WindowGroup)) {
                    console.log(
                        `.. has parent ${window_parent} but is not a ${Meta.WindowGroup.$gtype.name}, skipping`,
                    );
                    return;
                }

                window_parent.remove_child(window_actor);
                focus_window_actor.add_child(window_actor);

                // clean up to avoid a gnome-shell crash
                focus_window_actor.connect("destroy", (__) => {
                    focus_window_actor.remove_child(window_actor);
                    window.kill();
                });

                // make the overlay follow the workspace of the focus
                focus_window.connect("workspace-changed", (__) => {
                    const workspace = focus_window.get_workspace();
                    window.change_workspace(workspace);
                });

                // make the overlay always the first child of the focus
                // so it renders on top of the content
                focus_window.connect("focus", (__) => {
                    // TODO: how to make this instant?
                    GLib.timeout_add(0, 50, () => {
                        focus_window_actor.set_child_above_sibling(
                            window_actor,
                            null,
                        );
                        return false;
                    });
                });
                window.connect("focus", (__) => {
                    focus_window_actor.set_child_above_sibling(
                        window_actor,
                        null,
                    );
                });

                console.log(
                    `Attached "${window.title}" to "${focus_window.title}"`,
                );
            });
    }

    /**
     * @param {number} target_id
     * @param {number} target_pid
     * @param {string} target_title
     * @param {string} target_wm_class
     * @param {number} x
     * @param {number} y
     */
    SetPopupPosition(
        target_id,
        target_pid,
        target_title,
        target_wm_class,
        x,
        y,
    ) {
        // const popup_window = global
        //     .get_window_actors()
        //     .find((actor) => actor.meta_window.wm_class === APP_ID);
        // if (!popup_window) {
        //     logError(`Failed to find popup window with app ID "${APP_ID}"`);
        //     return;
        // }
        // /**
        //  * @param {Meta.Window} window
        //  */
        // const is_valid_window = (window) =>
        //     (target_id === 0 || target_id === window.get_id()) &&
        //     (target_pid === 0 || target_pid === window.get_pid()) &&
        //     (target_title === "" || target_title === window.title) &&
        //     (target_wm_class === "" || target_wm_class === window.wm_class);
        // const target_windows = global
        //     .get_window_actors()
        //     .filter((actor) => is_valid_window(actor.meta_window));
        // if (target_windows.length < 1) {
        //     logError(
        //         `Failed to find target window matching id ${target_id} pid ${target_pid} title "${target_title}" wm_class "${target_wm_class}"`,
        //     );
        //     return;
        // }
        // if (target_windows.length > 1) {
        //     logError(
        //         `Found ${target_windows.length} target windows matching id ${target_id} pid ${target_pid} title "${target_title}" wm_class "${target_wm_class}"`,
        //     );
        //     return;
        // }
        // const target_window = target_windows[0];
        // const target_rect = target_window.meta_window.get_frame_rect();
        // const [popup_x, popup_y] = [target_rect.x + x, target_rect.y + y];
        // popup_window.meta_window.raise();
        // popup_window.meta_window.move_frame(false, popup_x, popup_y);
    }
}
