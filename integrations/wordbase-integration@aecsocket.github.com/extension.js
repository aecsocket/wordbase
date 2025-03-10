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
import Clutter from "gi://Clutter";
import Meta from "gi://Meta";

import { Extension } from "resource:///org/gnome/shell/extensions/extension.js";
import * as Main from "resource:///org/gnome/shell/ui/main.js";
import * as MessageTray from "resource:///org/gnome/shell/ui/messageTray.js";
import * as PopupMenu from "resource:///org/gnome/shell/ui/popupMenu.js";
import * as Area from "./ui/dragArea.js";
import * as RichLabel from "./ui/richLabel.js";
import * as StickyScrollView from "./ui/stickyScrollView.js";
import * as Wordbase from "./wordbase.js";

/**
 * @typedef {Object} DialogBox
 * @property {Meta.WindowActor} parent
 * @property {St.Widget} root
 * @property {St.Widget} history_container
 */

export default class WordbaseIntegrationExtension extends Extension {
    /** @type {Gio.Settings} */
    _settings;
    /** @type {Soup.Session} */
    _soup;
    /** @type {MessageTray.Source} */
    _notification_source;
    /** @type {MessageTray.Notification?} */
    _status_notification;
    /** @type {Map<string, DialogBox>} */
    _dialog_boxes;
    /** @type {Wordbase.Client} */
    _wordbase;

    enable() {
        this._settings = this.getSettings();
        this._soup = new Soup.Session();
        this._dialog_boxes = new Map();

        this._settings.connect("changed::wordbase-url", (__, ___) => {
            this._connect_wordbase();
        });
        this._connect_wordbase();

        // GLib.timeout_add(0, 1000, () => {
        //     log(`should connect? ${!this._wordbase}`);
        //     if (!this._wordbase) {
        //         this._connect_wordbase();
        //     }
        //     return true;
        // });
    }

    disable() {
        this._settings = undefined;
        this._soup = undefined;

        this._notification_source?.destroy();
        this._notification_source = undefined;
        this._status_notification?.destroy();
        this._status_notification = undefined;

        this._dialog_boxes.forEach((dialog_box, _) => {
            dialog_box.root.destroy();
        });
        this._dialog_boxes = undefined;

        this._wordbase = undefined;
    }

    /**
     * @returns {MessageTray.Source}
     */
    _get_notification_source() {
        return MessageTray.getSystemSource();

        // if (!this._notification_source) {
        //     this._notification_source = new MessageTray.Source({
        //         title: this.metadata.name,
        //         iconName: "dialog-information",
        //         policy: new MessageTray.NotificationGenericPolicy(),
        //     });
        //     this._notification_source.connect("destroy", (_source) => {
        //         this._notification_source = null;
        //     });
        //     Main.messageTray.add(this._notification_source);
        // }

        // return this._notification_source;
    }

    /**
     * @param {MessageTray.Notification} notification
     */
    _show_status_notification(notification) {
        this._status_notification?.destroy(
            MessageTray.NotificationDestroyedReason.REPLACED,
        );
        this._get_notification_source().addNotification(notification);
        this._status_notification = notification;
        notification.connect("destroy", (__, ___) => {
            this._status_notification = null;
        });
    }

    _connect_wordbase() {
        const url = this._settings.get_string("wordbase-url");
        log(`Connecting to Wordbase at ${url}`);
        Wordbase.Client.connect(
            this._soup,
            url,
            (_err) => {},
            (client) => {
                this._wordbase = client;
                this._show_status_notification(
                    new MessageTray.Notification({
                        source: this._get_notification_source(),
                        title: _("Wordbase connected"),
                        urgency: MessageTray.Urgency.NORMAL,
                    }),
                );

                client.connection.connect("closed", (_source) => {
                    this._show_status_notification(
                        new MessageTray.Notification({
                            source: this._get_notification_source(),
                            title: _("Wordbase connection closed"),
                            urgency: MessageTray.Urgency.NORMAL,
                        }),
                    );
                    this._wordbase = null;
                });

                client.connection.connect("error", (_source) => {
                    this._show_status_notification(
                        new MessageTray.Notification({
                            source: this._get_notification_source(),
                            title: _("Wordbase connection lost"),
                            urgency: MessageTray.Urgency.NORMAL,
                        }),
                    );
                    this._wordbase = null;
                });

                client.on_hook_sentence = (message) => {
                    this._on_hook_sentence(message);
                };

                // const decoder = new TextDecoder();
                // connection.connect("message", (__, message_type, message) => {
                //     if (message_type != Soup.WebsocketDataType.TEXT) {
                //         return;
                //     }

                //     // TODO error handling
                //     /** @type {NewSentence} */
                //     const new_sentence = JSON.parse(decoder.decode(message.toArray()));
                //     this._on_new_sentence(new_sentence);
                // });
            },
        );
    }

    /**
     * @param {Wordbase.HookSentence} message
     */
    _on_hook_sentence(message) {
        const sentence = message.sentence.trim();

        let dialog_box = this._dialog_boxes.get(message.process_path);
        if (!dialog_box) {
            const target_window = global.display.get_focus_window();
            if (target_window) {
                /** @type {Meta.WindowActor} */
                const window_actor = target_window.get_compositor_private();
                dialog_box = this._new_dialog_box(window_actor);
            } else {
                dialog_box = this._new_dialog_box(global.window_group);
            }

            this._dialog_boxes.set(message.process_path, dialog_box);
        }

        const sentence_label = this._new_sentence_label(dialog_box, sentence);
        dialog_box.history_container.add_child(sentence_label);
    }

    /**
     * @param {Meta.WindowActor} parent
     * @returns {DialogBox}
     */
    _new_dialog_box(parent) {
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

        return { parent, root, history_container };
    }

    _setup_hover_opacity(widget) {
        const to_hover_opacity = new Clutter.PropertyTransition({
            property_name: "opacity",
            duration: 100,
            direction: Clutter.TimelineDirection.BACKWARD,
        });

        const update_hover_animation = () => {
            to_hover_opacity.start();
            if (
                to_hover_opacity.direction == Clutter.TimelineDirection.FORWARD
            ) {
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
        to_hover_opacity.set_from(
            this._settings.get_int("dialog-opacity-idle"),
        );
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
     * @param {DialogBox} dialog_box
     * @param {string} sentence
     * @returns {RichLabel.RichLabel}
     */
    _new_sentence_label(dialog_box, sentence) {
        const label = new RichLabel.RichLabel({
            text: sentence,
            x_expand: true,
            style_class: "sentence",
        });

        const encoder = new TextEncoder();
        const decoder = new TextDecoder();
        const sentence_bytes = encoder.encode(sentence);

        let last_char_pos = -1;
        label.clutter_text.connect(
            "motion-event",
            /**
             * @param {Clutter.Text} clutter_text
             * @param {Clutter.Event} event
             * @returns {boolean}
             */
            (clutter_text, event) => {
                const wordbase = this._wordbase;
                if (!wordbase) {
                    return Clutter.EVENT_PROPAGATE;
                }
                const primary_button = Clutter.ModifierType.BUTTON1_MASK;
                if ((event.get_state() & primary_button) !== 0) {
                    // user is dragging, don't interrupt
                    return Clutter.EVENT_PROPAGATE;
                }

                const [pointer_abs_x, pointer_abs_y] = event.get_coords();
                const [text_abs_x, text_abs_y] =
                    clutter_text.get_transformed_position();
                const [pointer_rel_x, pointer_rel_y] = [
                    pointer_abs_x - text_abs_x,
                    pointer_abs_y - text_abs_y,
                ];

                // THEY LIE! this is in BYTES, not CHARACTERS!
                // it sure is good that people will only ever use this extension
                // on ASCII text, and not something weird and foreign like
                // Japanese text! 😀😀😀😀😀😀😀
                const char_pos_bytes = clutter_text.coords_to_position(
                    pointer_rel_x,
                    pointer_rel_y,
                );
                const bytes_until_pos = sentence_bytes.slice(0, char_pos_bytes);
                const char_pos = decoder.decode(bytes_until_pos).length;

                const lookup_text = clutter_text.text.slice(
                    char_pos,
                    char_pos + wordbase.lookup_config.max_request_len,
                );
                log(
                    `pos = ${char_pos_bytes} | ${char_pos} / lookup = ${lookup_text}`,
                );

                clutter_text.grab_key_focus();
                clutter_text.set_selection(char_pos, char_pos + 3);

                return Clutter.EVENT_PROPAGATE;

                // if (char_pos == last_char_pos) {
                //     return Clutter.EVENT_PROPAGATE;
                // }
                // last_char_pos = char_pos;

                // wordbase.show_popup(
                //     {
                //         pid: 0, // TODO dialog_box.parent.meta_window.get_pid(),
                //         origin: [pointer_rel_x, pointer_rel_y],
                //         anchor: "BottomLeft",
                //         text: lookup_text,
                //     },
                //     (response) => {
                //         label.clutter_text.set_selection(
                //             char_pos,
                //             char_pos + response.chars_scanned,
                //         );
                //     },
                // );
                // return Clutter.EVENT_PROPAGATE;
            },
        );
        return label;
    }
}
