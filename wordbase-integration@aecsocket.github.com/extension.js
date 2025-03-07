/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 * @typedef {import("@girs/soup-3.0")}
 */

import Soup from "gi://Soup";
import GLib from "gi://GLib";
import Gio from "gi://Gio";
import St from "gi://St";
import Clutter from 'gi://Clutter';
import Meta from 'gi://Meta';

import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";
import * as Main from "resource:///org/gnome/shell/ui/main.js";
import * as MessageTray from 'resource:///org/gnome/shell/ui/messageTray.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as Area from "./ui/dragArea.js";
import * as RichLabel from "./ui/richLabel.js";
import * as DateTimeLabel from "./ui/dateTimeLabel.js";
import * as StickyScrollView from "./ui/stickyScrollView.js";

/**
 * @typedef {Object} NewSentence
 * @property {string} process_path
 * @property {string} sentence
 * 
 * @typedef {Object} DialogBox
 * @property {St.Widget} history_container
 * @property {WordbaseClient} wordbase
 * 
 * @typedef {Object} WordbaseClient
 * @property {Soup.WebsocketConnection} connection
 * @property {LookupConfig} lookup_config
 * 
 * @typedef {Object} LookupConfig
 * @property {number} max_request_len
 */

export default class WordbaseIntegrationExtension extends Extension {
    /** @type {Gio.Settings} */
    _settings;
    /** @type {Soup.Session} */
    _ws_session;
    /** @type {MessageTray.Source} */
    _notification_source;
    /** @type {Map<string, DialogBox>} */
    _dialog_boxes;
    /** @type {Soup.WebsocketConnection} */
    _texthooker;
    /** @type {MessageTray.Notification?} */
    _texthooker_notification;

    enable() {
        this._settings = this.getSettings();
        this._ws_session = new Soup.Session();
        this._notification_source = new MessageTray.Source({
            title: this.metadata.name,
            iconName: "dialog-information",
            policy: new MessageTray.NotificationGenericPolicy(),
        });
        this._notification_source.connect("destroy", _ => {
            this._notification_source = null;
        });
        Main.messageTray.add(this._notification_source);
        this._dialog_boxes = new Map();

        this._settings.connect("changed::texthooker-url", (__, ___) => {
            this._connect_texthooker();
        });
        this._settings.connect("changed::wordbase-url", (__, ___) => {
            this._connect_wordbase();
        });

        GLib.timeout_add(0, 1000, () => {
            if (!this._texthooker) {
                this._connect_texthooker();
            }
            return true;
        });
    }

    disable() {
        this._settings = undefined;
        this._ws_session = undefined;

        this._notification_source?.destroy();
        this._notification_source = undefined;

        this._dialog_boxes.forEach((dialog_box, _) => {
            dialog_box.root.destroy();
        });
        this._dialog_boxes = undefined;

        this._texthooker = undefined;
        this._texthooker_notification?.destroy();
        this._texthooker_notification = undefined;
    }

    /**
     * @param {MessageTray.Notification} notification 
     */
    _show_texthooker_notification(notification) {
        this._texthooker_notification?.destroy(MessageTray.NotificationDestroyedReason.REPLACED);
        this._notification_source.addNotification(notification);
        this._texthooker_notification = notification;
    }

    _connect_texthooker() {
        const url = this._settings.get_string("texthooker-url");
        this._ws_session.websocket_connect_async(
            new Soup.Message({
                method: "GET",
                uri: GLib.Uri.parse(url, GLib.UriFlags.NONE),
            }),
            "127.0.0.1",
            [],
            0,
            null,
            (session, res) => {
                /** @type {Soup.WebsocketConnection} */
                let connection;

                try {
                    connection = session.websocket_connect_finish(res);
                } catch (err) {
                    return;
                }
                this._texthooker = connection;

                this._show_texthooker_notification(new MessageTray.Notification({
                    source: this._notification_source,
                    title: _("Texthooker connected"),
                    urgency: MessageTray.Urgency.NORMAL,
                }));

                connection.connect("closed", (__) => {
                    this._show_texthooker_notification(new MessageTray.Notification({
                        source: this._notification_source,
                        title: _("Texthooker disconnected"),
                        urgency: MessageTray.Urgency.NORMAL,
                    }));
                    this._texthooker = null;
                });
                connection.connect("error", (__, err) => {
                    this._show_texthooker_notification(new MessageTray.Notification({
                        source: this._notification_source,
                        title: _("Texthooker connection lost"),
                        urgency: MessageTray.Urgency.NORMAL,
                    }));
                    this._texthooker = null;
                });

                const decoder = new TextDecoder();
                connection.connect("message", (__, message_type, message) => {
                    if (message_type != Soup.WebsocketDataType.TEXT) {
                        return;
                    }

                    // TODO error handling
                    /** @type {NewSentence} */
                    const new_sentence = JSON.parse(decoder.decode(message.toArray()));
                    this._on_new_sentence(new_sentence);
                })
            },
        );
    }

    /**
     * @param {function(WordbaseClient): void} callback 
     */
    _new_wordbase_client(callback) {
        const url = this._settings.get_string("wordbase-url");
        this._ws_session.websocket_connect_async(
            new Soup.Message({
                method: "GET",
                uri: GLib.Uri.parse(url, GLib.UriFlags.NONE),
            }),
            "127.0.0.1",
            [],
            0,
            null,
            (session, res) => {
                let connection;
                try {
                    connection = session.websocket_connect_finish(res);
                } catch (err) {
                    return;
                }

                // TODO fetch config
                this._wordbase = { connection, lookup_config: { max_request_len: 16 } };

                const decoder = new TextDecoder();
                connection.connect("message", (__, message_type, message) => {
                    if (message_type != Soup.WebsocketDataType.TEXT) {
                        return;
                    }

                    log(`msg ${decoder.decode(message.toArray())}`);
                });
            },
        );
    }

    /**
     * @param {NewSentence} new_sentence 
     */
    _on_new_sentence(new_sentence) {
        const sentence = new_sentence.sentence.trim();

        let dialog_box = this._dialog_boxes.get(new_sentence.process_path);
        if (!dialog_box) {
            const target_window = global.display.get_focus_window();
            if (target_window) {
                /** @type {Meta.WindowActor} */
                const window_actor = target_window.get_compositor_private();
                dialog_box = this._new_dialog_box(window_actor);
            } else {
                // TODO
                dialog_box = this._new_dialog_box(global.window_group);
                // END TODO
            }
            this._dialog_boxes.set(new_sentence.process_path, dialog_box);
        }

        const sentence_label = this._new_sentence_label(sentence);
        dialog_box.history_container.add_child(sentence_label);
    }

    /**
     * @param {Clutter.Actor} parent 
     * @returns {DialogBox}
     */
    _new_dialog_box(parent, callback) {
        const root = new Area.DragArea({
            style_class: "modal-dialog texthooker-dialog",
        });
        parent.add_child(root);
        root.set_position(100, 100);
        this._setup_hover_opacity(root);

        const overlapping = new St.Bin({
            x_expand: true,
            y_expand: true,
        });
        root.set_child(overlapping);

        const contents = new St.BoxLayout({
            x_expand: true,
            y_expand: true,
        });
        overlapping.add_child(contents);

        //
        // history
        //

        const history_scroll_view = new StickyScrollView.StickyScrollView({
            x_expand: true,
            y_expand: true,
            y_align: Clutter.ActorAlign.END,
            v_sticky: true,
            effect: new St.ScrollViewFade(),
        });
        contents.add_child(history_scroll_view);

        const history_container = new St.BoxLayout({
            x_expand: true,
            y_expand: true,
            layout_manager: new Clutter.BoxLayout({
                orientation: Clutter.Orientation.VERTICAL,
                spacing: 16, // TODO doesnt work?
            }),
        });
        history_scroll_view.set_child(history_container);

        const timestamp = GLib.DateTime.new_now(GLib.TimeZone.new_local()).format_iso8601();
        const dialog_session_header = new DateTimeLabel.DateTimeLabel({
            style_class: "session-header",
            timestamp,
        });
        history_container.add_child(dialog_session_header);

        this._new_wordbase_client(wordbase => {
            callback({
                root,
                history_container,
                wordbase,
            })
        });
    }

    _setup_hover_opacity(widget) {
        const to_hover_opacity = new Clutter.PropertyTransition({
            property_name: "opacity",
            duration: 100,
            direction: Clutter.TimelineDirection.BACKWARD,
        });

        const update_hover_animation = () => {
            to_hover_opacity.start();
            if (to_hover_opacity.direction == Clutter.TimelineDirection.FORWARD) {
                to_hover_opacity.advance(Number.MAX_SAFE_INTEGER);
            } else {
                to_hover_opacity.advance(0);
            }
        };

        this._settings.connect("changed::opacity-idle", (__, key) => {
            to_hover_opacity.set_from(this._settings.get_int(key));
            update_hover_animation();
        });
        this._settings.connect("changed::opacity-hover", (__, key) => {
            to_hover_opacity.set_to(this._settings.get_int(key));
            update_hover_animation();
        });
        to_hover_opacity.set_from(this._settings.get_int("dialog-opacity-idle"));
        to_hover_opacity.set_to(this._settings.get_int("dialog-opacity-hover"));

        widget.add_transition("to-hover-opacity", to_hover_opacity);
        widget.connect("enter-event", () => {
            to_hover_opacity.direction = Clutter.TimelineDirection.FORWARD;
            to_hover_opacity.start();
        });
        widget.connect("leave-event", () => {
            to_hover_opacity.direction = Clutter.TimelineDirection.BACKWARD;
            to_hover_opacity.start();
        });
    }

    /**
     * @param {string} sentence 
     * @param {function(): WordbaseClient?} get_wordbase
     * @returns {RichLabel.RichLabel}
     */
    _new_sentence_label(sentence, get_wordbase) {
        const label = new RichLabel.RichLabel({
            text: sentence,
            x_expand: true,
            style_class: "sentence",
        });
        label.clutter_text.connect(
            "motion-event",
            /**
             * @param {Clutter.Text} text
             * @param {Clutter.Event} event 
             * @returns {boolean}
             */
            (text, event) => {
                const wordbase = get_wordbase();
                if (!wordbase) {
                    return;
                }

                const [pointer_abs_x, pointer_abs_y] = event.get_coords();
                const [text_abs_x, text_abs_y] = text.get_transformed_position();
                const [pointer_rel_x, pointer_rel_y] = [pointer_abs_x - text_abs_x, pointer_abs_y - text_abs_y];
                const char_pos = text.coords_to_position(pointer_rel_x, pointer_rel_y);

                const lookup_text = text.text.slice(
                    char_pos,
                    char_pos + this._wordbase.lookup_config.max_request_len,
                );

                wordbase.connection.send_text(JSON.stringify({
                    type: "Lookup",
                    text: lookup_text,
                    wants_html: false,
                }));
            },
        );
        return label;
    }
}
