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
    const service = new IntegrationService();
    ext._service_export = Gio.DBusExportedObject.wrapJSObject(
        INTERFACE,
        service,
    );
    service._impl = ext._service_export;
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
        <method name="OverlayOnWindow">
            <arg direction="in" type="t" name="parent_id"/>
            <arg direction="in" type="t" name="overlay_id"/>
        </method>
        <method name="MovePopupToWindow">
            <arg direction="in" type="t" name="moved_id"/>
            <arg direction="in" type="t" name="to_id"/>
            <arg direction="in" type="s" name="to_title"/>
            <arg direction="in" type="s" name="to_wm_class"/>
            <arg direction="in" type="i" name="offset_x"/>
            <arg direction="in" type="i" name="offset_y"/>
        </method>
        <signal name="CloseOverlay">
            <arg type="t" name="overlay_id"/>
        </signal>
    </interface>
</node>`;

// TODO: this is what happens when you use C
// you get constant segfaults for no reason
// fuckkkkkk
// i commented out the `move_frame` calls for now
// https://gitlab.gnome.org/GNOME/mutter/-/issues/1600
// https://gitlab.gnome.org/GNOME/mutter/-/blob/main/src/wayland/meta-window-wayland.c?ref_type=heads
//
// It happens sometimes when you add an overlay to a window, and then move the parent window.
// Specifically when I spawned an overlay on top of my fullscreen browser, then start dragging
// the browser window, mutter crashes

class IntegrationService {
    /** @type {Gio.DBusExportedObject} */
    _impl;

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
     * @param {Gio.DBusMethodInvocation} invocation
     * @param {GLib.Variant} params
     * @returns {number}
     */
    GetAppWindowId(invocation, params) {
        const [title] = params.deep_unpack();
        const window = global
            .get_window_actors()
            .map((window_actor) => window_actor.meta_window)
            .find(
                (window) =>
                    window.gtk_application_id === APP_ID &&
                    window.get_title() === title,
            );
        if (!window) {
            invocation.return_dbus_error(
                Gio.DBusError.INVALID_ARGS,
                `no Wordbase window with title "${title}", windows:\n${window_debug_info()}`,
            );
            return;
        }
        return window.get_id();
    }

    /**
     * @param {number} parent_id
     * @param {number} overlay_id
     */
    OverlayOnWindow(parent_id, overlay_id) {
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

        // init logic

        const parent_rect = parent_window.get_frame_rect();
        let handler_id = null;
        handler_id = overlay_window.connect("shown", (__) => {
            if (!handler_id) {
                return;
            }
            overlay_window.disconnect(handler_id);
            handler_id = null;

            overlay_window.focus(global.get_current_time());
            overlay_window.raise();
            overlay_window.move_frame(false, parent_rect.x, parent_rect.y);
            if (parent_window.is_fullscreen()) {
                overlay_window.make_above();
            }
        });

        // actual logic
        // note: for each `parent_window.connect` here (1),
        // we MUST also add a `overlay_window.connect("destroy")`
        // which cleans up that (1)
        // we do that using `parent_connect`
        const parent_window_connect = (id, callback) => {
            const handler_id = parent_window.connect(id, callback);
            overlay_actor.connect("destroy", (__) => {
                parent_window.disconnect(handler_id);
            });
        };
        const parent_actor_connect = (id, callback) => {
            const handler_id = parent_actor.connect(id, callback);
            overlay_actor.connect("destroy", (__) => {
                parent_actor.disconnect(handler_id);
            });
        };

        // make the overlay follow the parent's position
        let [parent_last_x, parent_last_y] = [
            parent_window.get_frame_rect().x,
            parent_window.get_frame_rect().y,
        ];
        parent_window_connect("position-changed", (__) => {
            const [parent_now_x, parent_now_y] = [
                parent_window.get_frame_rect().x,
                parent_window.get_frame_rect().y,
            ];
            const [parent_delta_x, parent_delta_y] = [
                parent_now_x - parent_last_x,
                parent_now_y - parent_last_y,
            ];
            parent_last_x = parent_now_x;
            parent_last_y = parent_now_y;

            const [overlay_now_x, overlay_now_y] = [
                overlay_window.get_frame_rect().x,
                overlay_window.get_frame_rect().y,
            ];
            const [overlay_new_x, overlay_new_y] = [
                overlay_now_x + parent_delta_x,
                overlay_now_y + parent_delta_y,
            ];
            overlay_window.move_frame(false, overlay_new_x, overlay_new_y);
        });

        // make the overlay follow the parent's workspace
        parent_window_connect("workspace-changed", (__) => {
            const workspace = parent_window.get_workspace();
            if (workspace) {
                overlay_window.change_workspace(workspace);
            }
        });
        overlay_window.connect("workspace-changed", (__) => {
            const workspace = parent_window.get_workspace();
            if (workspace) {
                overlay_window.change_workspace(workspace);
            }
        });

        parent_window_connect("focus", (__) => {
            overlay_window.raise();
        });
        parent_window_connect("raised", (__) => {
            overlay_window.raise();
        });

        parent_window_connect("notify::fullscreen", (__) => {
            if (parent_window.is_fullscreen()) {
                overlay_window.make_above();
            } else {
                overlay_window.unmake_above();
            }
        });

        parent_actor_connect("destroy", (__) => {
            console.log(
                `"${overlay_window.title}" destroyed, closing ${overlay_id}`,
            );
            this._impl.emit_signal(
                "CloseOverlay",
                new GLib.Variant("(t)", [overlay_id]),
            );
        });

        console.log(
            `Attached "${overlay_window.title}" to "${parent_window.title}"`,
        );
    }

    /**
     * @param {number} moved_id
     * @param {number} to_id
     * @param {string} to_title
     * @param {string} to_wm_class
     * @param {number} offset_x
     * @param {number} offset_y
     */
    MovePopupToWindow(
        moved_id,
        to_id,
        to_title,
        to_wm_class,
        offset_x,
        offset_y,
    ) {
        // find moved window

        const moved_actor = global
            .get_window_actors()
            .find((window) => window.meta_window.get_id() === moved_id);
        if (!moved_actor) {
            throw new Error(
                `no window with ID ${moved_id}, windows:\n${window_debug_info()}`,
            );
        }
        const moved_window = moved_actor.meta_window;
        if (moved_window.gtk_application_id !== APP_ID) {
            throw new Error(
                `window GTK application ID is ${moved_window.gtk_application_id}, not ${APP_ID}`,
            );
        }

        // find to window

        /**
         * @param {Meta.Window} window
         */
        const is_to_window = (window) =>
            (to_id === 0 || to_id === window.get_id()) &&
            (to_title === "" || to_title === window.title) &&
            (to_wm_class === "" || to_wm_class === window.wm_class);
        const to_actors = global
            .get_window_actors()
            .filter((actor) => is_to_window(actor.meta_window));
        if (to_actors.length < 1) {
            throw new Error(
                `no window matching filter (id=${to_id}, to_title=${to_title}, to_wm_class=${to_wm_class}), windows:\n${window_debug_info()}`,
            );
            return;
        }
        if (to_actors.length > 1) {
            throw new Error(
                `found ${to_actors.length} matching (id=${to_id}, to_title=${to_title}, to_wm_class=${to_wm_class}), but only 1 must match; windows:\n${window_debug_info()}`,
            );
            return;
        }

        // move the window

        moved_window.focus(global.get_current_time());
        moved_window.raise();
        const to_actor = to_actors[0];
        const to_window = to_actor.meta_window;
        const to_rect = to_window.get_frame_rect();
        const [moved_x, moved_y] = [to_rect.x + offset_x, to_rect.y + offset_y];

        // when we move the window, if we've *just* shown and presented the window
        // (made it visible on the GTK side), then it might not be ready to move to
        // its new position yet
        //
        // to get around this, we move it 2 times:
        // - right now
        // - after (or if) it's shown (presented and ready to move)
        //
        // if the window doesn't end up `shown` soon, then we won't do the 2nd move
        moved_window.move_frame(false, moved_x, moved_y);
        let handler_id = null;
        handler_id = moved_window.connect("shown", (__) => {
            if (!handler_id) {
                return;
            }
            moved_window.disconnect(handler_id);
            handler_id = null;

            moved_window.move_frame(false, moved_x, moved_y);
            // even though we've just raised the popup window, it's not guaranteed
            // to be on top if we're in a fullscreen window
            // so we force it to be always on top
            // this shouldn't impact the user experience, since as soon as the unfocus,
            // the popup window will hide anyway
            moved_window.make_above();
        });
        GLib.timeout_add(0, 100, () => {
            if (!handler_id) {
                return;
            }
            moved_window.disconnect(handler_id);
            handler_id = null;

            return false;
        });
    }
}

function window_debug_info() {
    return global
        .get_window_actors()
        .map((window_actor) => window_actor.meta_window)
        .map(
            (window) =>
                `- "${window.title}" -> ${window.get_id()}
    GTK app ID: ${window.gtk_application_id}
    WM_CLASS: ${window.get_wm_class()}`,
        )
        .join("\n");
}
