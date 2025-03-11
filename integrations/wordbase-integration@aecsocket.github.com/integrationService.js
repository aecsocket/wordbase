/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import Gio from "gi://Gio";
import Meta from "gi://Meta";

import * as Main from "resource:///org/gnome/shell/ui/main.js";

const APP_ID = "com.github.aecsocket.WordbasePopup";
const BUS_NAME = "com.github.aecsocket.WordbaseIntegration";
const INTERFACE_NAME = "/com/github/aecsocket/WordbaseIntegration";
const INTERFACE = `
<node>
    <interface name="com.github.aecsocket.WordbaseIntegration">
        <method name="SetPopupPosition">
            <arg type="t" direction="in" name="target_id"/>
            <arg type="u" direction="in" name="target_pid"/>
            <arg type="s" direction="in" name="target_title"/>
            <arg type="s" direction="in" name="target_wm_class"/>
            <arg type="i" direction="in" name="x"/>
            <arg type="i" direction="in" name="y"/>
        </method>
    </interface>
</node>`;

/** @type {number} */
let bus_owner = null;

/** @type {Gio.DBusExportedObject} */
let exported_object = null;

export function enable() {
    disable();
    bus_owner = Gio.bus_own_name(
        Gio.BusType.SESSION,
        BUS_NAME,
        Gio.BusNameOwnerFlags.NONE,
        on_bus_acquired,
        on_name_acquired,
        on_name_lost,
    );
}

export function disable() {
    if (bus_owner) {
        Gio.bus_unown_name(bus_owner);
        bus_owner = null;
    }
    if (exported_object) {
        exported_object.unexport();
        exported_object = null;
    }
}

function on_name_acquired(_connection, _name) {}

function on_name_lost(connection, _name) {
    disable();
}

/**
 * @param {Gio.DBusConnection} connection
 * @param {string} name
 */
function on_bus_acquired(connection, name) {
    let service = new IntegrationService();
    exported_object = Gio.DBusExportedObject.wrapJSObject(INTERFACE, service);
    exported_object.export(connection, INTERFACE_NAME);
}

class IntegrationService {
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
        const popup_window = global
            .get_window_actors()
            .find((actor) => actor.meta_window.wm_class === APP_ID);
        if (!popup_window) {
            logError(`Failed to find popup window with app ID "${APP_ID}"`);
            return;
        }

        /**
         * @param {Meta.Window} window
         */
        const is_valid_window = (window) =>
            (target_id === 0 || target_id === window.get_id()) &&
            (target_pid === 0 || target_pid === window.get_pid()) &&
            (target_title === "" || target_title === window.title) &&
            (target_wm_class === "" || target_wm_class === window.wm_class);

        const target_windows = global
            .get_window_actors()
            .filter((actor) => is_valid_window(actor.meta_window));
        if (target_windows.length < 1) {
            logError(
                `Failed to find target window matching id ${target_id} pid ${target_pid} title "${target_title}" wm_class "${target_wm_class}"`,
            );
            return;
        }
        if (target_windows.length > 1) {
            logError(
                `Found ${target_windows.length} target windows matching id ${target_id} pid ${target_pid} title "${target_title}" wm_class "${target_wm_class}"`,
            );
            return;
        }
        const target_window = target_windows[0];

        const target_rect = target_window.meta_window.get_frame_rect();
        const [popup_x, popup_y] = [target_rect.x + x, target_rect.y + y];

        popup_window.meta_window.raise();
        popup_window.meta_window.move_frame(false, popup_x, popup_y);
    }
}
