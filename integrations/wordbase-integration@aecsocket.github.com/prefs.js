/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import Gio from "gi://Gio";
import Adw from "gi://Adw";
import Gtk from "gi://Gtk";
import Pango from "gi://Pango";

import {
    ExtensionPreferences,
    gettext as _,
} from "resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js";

export default class WordbaseIntegrationPreferences extends ExtensionPreferences {
    /**
     * @param {Adw.PreferencesWindow} window
     */
    fillPreferencesWindow(window) {
        const ui = Gtk.Builder.new_from_file(`${this.path}/Prefs.ui`);
        window.add(ui.get_object("general_page"));

        const settings = this.getSettings();
        window._settings = settings;

        const bind = (widget_id, setting_key, property_key) => {
            const widget = ui.get_object(widget_id);
            settings.bind(
                setting_key,
                widget,
                property_key,
                Gio.SettingsBindFlags.DEFAULT,
            );
        };

        bind("wordbase_url", "wordbase-url", "text");
        bind("dialog_opacity_idle", "dialog-opacity-idle", "value");
        bind("dialog_opacity_hover", "dialog-opacity-hover", "value");
        bind("dialog_popup_x_offset", "dialog-popup-x-offset", "value");
        bind("dialog_popup_y_offset", "dialog-popup-y-offset", "value");

        /** @type {Adw.PreferencesRow} */
        const dialog_font_row = ui.get_object("dialog_font_row");
        /** @type {Gtk.Label} */
        const dialog_font_label = ui.get_object("dialog_font_label");

        dialog_font_row.connect("activate", (__) => {
            const dialog = new Gtk.FontDialog();
            dialog.choose_font_and_features(
                window,
                null,
                null,
                (dialog, res, ___) => {
                    const [selected, font_description, idk, language] =
                        dialog.choose_font_and_features_finish(res);

                    if (!selected) {
                        return;
                    }

                    dialog_font_label.set_text(idk);
                },
            );
        });
    }
}
