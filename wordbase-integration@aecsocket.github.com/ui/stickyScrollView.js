/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import St from "gi://St";
import GObject from 'gi://GObject';

export const StickyScrollView = GObject.registerClass({
    Properties: {
        "h_sticky": GObject.ParamSpec.boolean(
            "h_sticky", null, null,
            GObject.ParamFlags.READWRITE,
            false,
        ),
        "v_sticky": GObject.ParamSpec.boolean(
            "v_sticky", null, null,
            GObject.ParamFlags.READWRITE,
            false,
        ),
    },
}, class StickyScrollView extends St.ScrollView {
    /** @type {number} */
    _last_upper_h;
    /** @type {number} */
    _last_upper_v;

    _init(params) {
        super._init(params);
        this._last_upper_h = this.hadjustment.upper;
        this._last_upper_v = this.vadjustment.upper;
        this.hadjustment.connect(
            "changed",
            (adjustment) => {
                if (adjustment.value + adjustment.page_size >= this._last_upper_h) {
                    adjustment.value = adjustment.upper;
                }
                this._last_upper_h = adjustment.upper;
            }
        );
        this.vadjustment.connect(
            "changed",
            (adjustment) => {
                if (adjustment.value + adjustment.page_size >= this._last_upper_v) {
                    adjustment.value = adjustment.upper;
                }
                this._last_upper_v = adjustment.upper;
            }
        );
    }
});
