/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import Gio from "gi://Gio";
import GLib from "gi://GLib";
import Meta from "gi://Meta";

import * as Main from "resource:///org/gnome/shell/ui/main.js";
import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";

export default class WordbaseIntegrationExtension extends Extension {
    /** @type {number} */
    _bus_owner;
    /** @type {Gio.DBusExportedObject} */
    _service_export;
    /** @type {number} */
    _bus_filter;

    enable() {
        this._bus_owner = Gio.bus_own_name(
            Gio.BusType.SESSION,
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            (conn, name) => on_bus_acquired(this, conn, name),
            (conn, name) => on_name_acquired(this, conn, name),
            (conn, name) => on_name_lost(this, conn, name),
        );

        // const dbus = Gio.DBus.session;
        // const objectManagerProxy = new Gio.DBusProxy({
        //     g_connection: dbus,
        //     g_interface_name: "org.freedesktop.DBus.ObjectManager",
        //     g_name: "org.gnome.Mutter.ScreenCast",
        //     g_object_path: "/org/gnome/Mutter/ScreenCast",
        //     g_flags: Gio.DBusProxyFlags.NONE,
        // });

        // objectManagerProxy.connectSignal(
        //     "InterfacesAdded",
        //     (connection, sender, path, iface, signal, params) => {
        //         const [objectPath, interfaces] = params.deepUnpack();
        //         log(`TODO: New object created: ${objectPath}`);
        //         if (
        //             objectPath.startsWith(
        //                 "/org/gnome/Mutter/ScreenCast/Session",
        //             )
        //         ) {
        //             log(`TODO: New ScreenCast session created: ${objectPath}`);
        //         }
        //     },
        // );
        // log(`TODO!! Made signal handler`);

        // console.log(
        //     `TODO!! sess = ${Gio.DBus.session} / add filter = ${Gio.DBus.session.add_filter}`,
        // );
        // Gio.DBus.session.add_filter((c, m, i) => null);

        // this._bus_filter = Gio.DBus.session.add_filter(
        //     (connection, message, incoming) => {
        //         console.log(`got a msg! ${message}`);

        //         return false;
        //     },
        // );

        // <https://gitlab.gnome.org/GNOME/xdg-desktop-portal-gnome/-/blob/main/src/screencastdialog.c>
        // <https://gitlab.gnome.org/GNOME/xdg-desktop-portal-gnome/-/blob/main/src/screencast.c#L633>

        // from <https://gitlab.gnome.org/GNOME/gnome-shell/-/blob/main/js/ui/status/remoteAccess.js#L16>
        // global.backend
        //     .get_remote_access_controller()
        //     .connect("new-handle", (handle) => {
        //         // hmm...
        //     });

        // global.get_window_actors().forEach((actor) => {
        //     actor.meta_window.cast;
        // });
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
        if (this._bus_filter) {
            Gio.DBus.session.remove_filter(this._bus_filter);
            this._bus_filter = undefined;
        }
    }

    /**
     * @param {Gio.DBusConnection} connection
     * @param {Gio.DBusMessage} message
     * @param {boolean} incoming
     * @returns {Gio.DBusMessage}
     */
    dbus_filter(connection, message, incoming) {
        return message;
    }
}

/**
 * @param {Gio.DBusConnection} connection
 * @param {Gio.DBusMessage} message
 * @param {boolean} incoming
 * @returns {Gio.DBusMessage}
 */
function dbus_filter(connection, message, incoming) {
    return message;

    // if (
    //     !incoming ||
    //     message.get_message_type() !== Gio.DBusMessageType.METHOD_CALL ||
    //     message.get_interface() !== "org.gnome.Mutter.ScreenCast.Session" ||
    //     message.get_member() !== "RecordWindow"
    // ) {
    //     return message;
    // }

    // const [args_variant] = message.get_body().deep_unpack(); // a{sv}
    // const args_dict = args_variant.deep_unpack();
    // const window_id = args_dict["window-id"].get_uint64();
    // console.log(`OMG FOUND!!! window id = ${window_id}`);

    // return message;
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
        <signal name="CloseOverlay">
            <arg type="t" name="overlay_id"/>
        </signal>
        <method name="ScreenshotOverlayParent">
            <arg direction="in" type="t" name="overlay_id"/>
            <arg direction="out" type="ay" name="screenshot"/>
        </method>
        <method name="MovePopupToWindow">
            <arg direction="in" type="t" name="moved_id"/>
            <arg direction="in" type="t" name="to_id"/>
            <arg direction="in" type="s" name="to_title"/>
            <arg direction="in" type="s" name="to_wm_class"/>
            <arg direction="in" type="i" name="offset_nw_x"/>
            <arg direction="in" type="i" name="offset_nw_y"/>
            <arg direction="in" type="i" name="offset_se_x"/>
            <arg direction="in" type="i" name="offset_se_y"/>
        </method>
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
            return new Error(
                `no Wordbase window with title "${title}", windows:\n${window_debug_info()}`,
            );
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
            return new Error(
                `no parent window with ID ${parent_id}, windows:\n${window_debug_info()}`,
            );
        }
        const parent_window = parent_actor.meta_window;

        const overlay_actor = global
            .get_window_actors()
            .find((window) => window.meta_window.get_id() === overlay_id);
        if (!overlay_actor) {
            return new Error(
                `no overlay window with ID ${parent_id}, windows:\n${window_debug_info()}`,
            );
        }
        const overlay_window = overlay_actor.meta_window;
        if (overlay_window.gtk_application_id !== APP_ID) {
            return new Error(
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
     * @param {number} overlay_id
     */
    ScreenshotOverlayParent(overlay_id) {}

    /**
     * @param {number} moved_id
     * @param {number} to_id
     * @param {string} to_title
     * @param {string} to_wm_class
     * @param {number} offset_nw_x
     * @param {number} offset_nw_y
     * @param {number} offset_se_x
     * @param {number} offset_se_y
     */
    MovePopupToWindow(
        moved_id,
        to_id,
        to_title,
        to_wm_class,
        offset_nw_x,
        offset_nw_y,
        offset_se_x,
        offset_se_y,
    ) {
        // find moved window

        const moved_actor = global
            .get_window_actors()
            .find((window) => window.meta_window.get_id() === moved_id);
        if (!moved_actor) {
            return new Error(
                `no window with ID ${moved_id}, windows:\n${window_debug_info()}`,
            );
        }
        const moved_window = moved_actor.meta_window;
        if (moved_window.gtk_application_id !== APP_ID) {
            return new Error(
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
            return new Error(
                `no window matching filter (id=${to_id}, to_title=${to_title}, to_wm_class=${to_wm_class}), windows:\n${window_debug_info()}`,
            );
        }
        if (to_actors.length > 1) {
            return new Error(
                `found ${to_actors.length} matching (id=${to_id}, to_title=${to_title}, to_wm_class=${to_wm_class}), but only 1 must match; windows:\n${window_debug_info()}`,
            );
        }
        const to_actor = to_actors[0];
        const to_window = to_actor.meta_window;

        // move the window

        moved_window.focus(global.get_current_time());
        moved_window.raise();

        const to_rect = to_window.get_frame_rect();
        const moved_rect = moved_window.get_frame_rect();
        const monitor = to_window.get_monitor();
        if (monitor < 0) {
            return new Error("to window has no monitor");
        }
        const monitor_rect = global.display.get_monitor_geometry(monitor);
        // positioning logic:
        //
        // - for the X axis:
        //   - align the moved window's west edge with the origin's west edge
        //   - TODO: RTL languages?
        // - for the Y axis:
        //   - if we have enough space to put the window below the south edge:
        //     - align the window's north edge with the origin's south edge
        //   - if there's not enough space:
        //     - align the window's south edge with the origin's north edge

        const moved_x = to_rect.x + offset_nw_x;

        const target_moved_bottom_y =
            to_rect.y + offset_se_y + moved_rect.height;
        let moved_y;
        if (target_moved_bottom_y < monitor_rect.y + monitor_rect.height) {
            moved_y = to_rect.y + offset_se_y;
        } else {
            moved_y = to_rect.y + offset_nw_y - moved_rect.height;
        }

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
        // even though we've just raised the popup window, it's not guaranteed
        // to be on top if we're in a fullscreen window
        // so we force it to be always on top
        // this shouldn't impact the user experience, since as soon as the window is unfocused,
        // the popup window will hide anyway
        moved_window.make_above();
        let handler_id = null;
        handler_id = moved_window.connect("shown", (__) => {
            if (!handler_id) {
                return;
            }
            moved_window.disconnect(handler_id);
            handler_id = null;

            moved_window.move_frame(false, moved_x, moved_y);
            // ditto
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
