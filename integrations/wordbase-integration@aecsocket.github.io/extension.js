/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import Gio from "gi://Gio";
import GLib from "gi://GLib";
import Meta from "gi://Meta";

import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";

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
        <method name="GetFocusedWindowId">
            <arg direction="out" type="t" name="id"/>
        </method>
        <method name="GetAppWindowId">
            <arg direction="in" type="s" name="title"/>
            <arg direction="out" type="t" name="id"/>
        </method>
        <method name="AffixToWindow">
            <arg direction="in" type="t" name="parent_id"/>
            <arg direction="in" type="t" name="overlay_id"/>
        </method>
        <method name="MoveToWindow">
            <arg direction="in" type="t" name="target_id"/>
            <arg direction="in" type="t" name="to_id"/>
            <arg direction="in" type="u" name="to_pid"/>
            <arg direction="in" type="s" name="to_title"/>
            <arg direction="in" type="s" name="to_wm_class"/>
            <arg direction="in" type="i" name="offset_x"/>
            <arg direction="in" type="i" name="offset_y"/>
        </method>
    </interface>
</node>`;

class IntegrationService {
    /**
     * @returns {number}
     */
    GetFocusedWindowId() {
        const focus_window = global.display.focus_window;
        if (!focus_window) {
            return 0;
        }
        return focus_window.get_id();
    }

    /**
     * @param {string} title
     * @returns {number}
     */
    GetAppWindowId(title) {
        const window = global
            .get_window_actors()
            .map((window_actor) => window_actor.meta_window)
            .find(
                (window) =>
                    window.gtk_application_id === APP_ID &&
                    window.get_title() === title,
            );
        if (!window) {
            throw new Error(
                `no Wordbase window with title "${title}", windows:\n${window_debug_info()}`,
            );
        }
        return window.get_id();
    }

    /**
     * @param {number} parent_id
     * @param {number} overlay_id
     */
    AffixToWindow(parent_id, overlay_id) {
        const parent_actor = global
            .get_window_actors()
            .find((window) => window.meta_window.get_id() === parent_id);
        if (!parent_actor) {
            throw new Error(
                `no parent window with ID ${parent_id}, windows:\n${window_debug_info()}`,
            );
        }
        const parent_window = parent_actor.meta_window;

        const overlay_actor = global
            .get_window_actors()
            .find((window) => window.meta_window.get_id() === overlay_id);
        if (!overlay_actor) {
            throw new Error(
                `no overlay window with ID ${parent_id}, windows:\n${window_debug_info()}`,
            );
        }
        const overlay_window = overlay_actor.meta_window;
        if (overlay_window.gtk_application_id !== APP_ID) {
            throw new Error(
                `overlay window GTK application ID is ${overlay_window.gtk_application_id}, not ${APP_ID}`,
            );
        }

        overlay_actor.get_parent().remove_child(overlay_actor);
        parent_actor.add_child(overlay_actor);
        overlay_actor.set_position(0, 0);

        // clean up to avoid a gnome-shell crash
        parent_actor.connect("destroy", (__) => {
            parent_actor.remove_child(overlay_actor);
            overlay_window.kill();
        });

        // make the overlay follow the workspace of the focus
        parent_window.connect("workspace-changed", (__) => {
            // TODO: how to make this instant?
            GLib.timeout_add(0, 50, () => {
                const workspace = parent_window.get_workspace();
                // workspace may be null
                if (workspace) {
                    overlay_window.change_workspace(workspace);
                }
                return false;
            });
        });

        // make the overlay always the first child of the focus
        // so it renders on top of the content
        parent_window.connect("focus", (__) => {
            // TODO: how to make this instant?
            GLib.timeout_add(0, 50, () => {
                if (parent_window.is_alive) {
                    console.log(
                        `parent window focus = PARENT: ${parent_actor} / OVERLAY: ${overlay_actor} / OVERLAY PARENT = ${overlay_actor.get_parent()}`,
                    );

                    parent_actor.set_child_above_sibling(overlay_actor, null);
                    return false;
                }
            });
        });
        overlay_window.connect("focus", (__) => {
            if (parent_window.is_alive) {
                console.log(
                    `overlay window focus = PARENT: ${parent_actor} / OVERLAY: ${overlay_actor} / OVERLAY PARENT = ${overlay_actor.get_parent()}`,
                );

                parent_actor.set_child_above_sibling(overlay_actor, null);
            }
        });

        console.log(
            `Attached "${overlay_window.title}" to "${parent_window.title}"`,
        );
    }

    /**
     * @param {number} target_id
     * @param {number} to_id
     * @param {number} to_pid
     * @param {string} to_title
     * @param {string} to_wm_class
     * @param {number} offset_x
     * @param {number} offset_y
     */
    SetPopupPosition(
        target_id,
        to_id,
        to_pid,
        to_title,
        to_wm_class,
        offset_x,
        offset_y,
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

function window_debug_info() {
    return global
        .get_window_actors()
        .map((window_actor) => window_actor.meta_window)
        .map(
            (window) =>
                `- "${window.title}" (GTK ID: ${window.gtk_application_id}) -> ${window.get_id()}`,
        )
        .join("\n");
}
