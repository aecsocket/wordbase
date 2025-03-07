/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import St from "gi://St";
import GObject from 'gi://GObject';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import GnomeDesktop from 'gi://GnomeDesktop';

export const DateTimeLabel = GObject.registerClass({
    Properties: {
        "timestamp": GObject.ParamSpec.string(
            "timestamp", null, null,
            GObject.ParamFlags.READWRITE,
            "",
        ),
    }
}, class DateLabel extends St.Label {
    /** @type {GnomeDesktop.WallClock} */
    _wall_clock;
    /** @type {Gio.Settings} */
    _desktop_settings;

    _init(params) {
        super._init(params);
        this._wall_clock = new GnomeDesktop.WallClock();
        this._desktop_settings = Gio.Settings.new("org.gnome.desktop.interface");
        // TODO these dont change
        this._desktop_settings.connect("changed::clock-format", (__, ___) => this._update());
        this._desktop_settings.connect("changed::clock-show-weekday", (__, ___) => this._update());
        this._desktop_settings.connect("changed::clock-show-date", (__, ___) => this._update());
        this._desktop_settings.connect("changed::clock-show-seconds", (__, ___) => this._update());
        // TODO bind timestamp update
        this._update();
    }

    _update() {
        const date_time = GLib.DateTime.new_from_iso8601(
            this.timestamp,
            GLib.TimeZone.new_local(),
        );
        const clock_format = this._desktop_settings.get_enum("clock-format");
        const show_weekday = this._desktop_settings.get_boolean("clock-show-weekday");
        const show_full_date = this._desktop_settings.get_boolean("clock-show-date");
        const show_seconds = this._desktop_settings.get_boolean("clock-show-seconds");
        this.text = this._wall_clock.string_for_datetime(
            date_time,
            clock_format,
            show_weekday,
            show_full_date,
            show_seconds,
        );
    }
})
