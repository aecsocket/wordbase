/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import Gio from "gi://Gio";

const APP_ID = "com.github.aecsocket.WordbasePopup";
const BUS_NAME = "com.github.aecsocket.WordbaseIntegration";
const INTERFACE_NAME = "/com/github/aecsocket/WordbaseIntegration";
const INTERFACE = `
<node>
    <interface name="com.github.aecsocket.WordbaseIntegration">
        <method name="SetPopupPosition">
            <arg type="s" direction="in" name="target"/>
            <arg type="u" direction="in" name="x"/>
            <arg type="u" direction="in" name="y"/>
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
    /*
    gdbus call --session \
        -d com.github.aecsocket.WordbaseIntegration \
        -o /com/github/aecsocket/WordbaseIntegration \
        -m com.github.aecsocket.WordbaseIntegration.SetPopupPosition \
        test 4 4 */

    /**
     * @param {string} target
     * @param {number} x
     * @param {number} y
     */
    SetPopupPosition(target, x, y) {
        log(`set popup position ${target} / ${x} / ${y}`);

        const popup_window = global
            .get_window_actors()
            .find((actor) => actor.meta_window.wm_class === APP_ID);
        if (!popup_window) {
            log(`Failed to find popup window "${APP_ID}"`);
            return;
        }

        const target_window = global
            .get_window_actors()
            .find((actor) => actor.meta_window.wm_class === target);
        if (!target_window) {
            log(`Failed to find target window "${target}"`);
            return;
        }

        const target_rect = target_window.meta_window.get_frame_rect();
        const [popup_x, popup_y] = [target_rect.x + x, target_rect.y + y];

        popup_window.meta_window.raise();
        popup_window.meta_window.move_frame(false, popup_x, popup_y);
    }
}
