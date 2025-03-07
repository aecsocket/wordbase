/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import St from "gi://St";
import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import Pango from 'gi://Pango';

/**
 * Displays a block with text inside, but which cannot be edited.
 * 
 * {@link St.Entry} provides text rendering and some extra goodies like clipboard handling.
 * However, it's designed for text *editing*, and setting the underlying {@link Clutter.Text}
 * doesn't fully disable editing (i.e. via clipboard). This wrapper prevents all text editing.
 */
// St.Entry logic:
// <https://github.com/GNOME/gnome-shell/blob/26b11f0fe54b91b751633a6da5662271552ce1e5/src/st/st-entry.c#L648>
export const RichLabel = GObject.registerClass({}, class RichLabel extends St.Entry {
    _init(params) {
        super._init(params);
        this.clutter_text.editable = false;
        this.clutter_text.line_wrap = true;
        this.clutter_text.single_line_mode = false;
        this.clutter_text.line_wrap_mode = Pango.WrapMode.WORD;
    }

    /**
     * @param {Clutter.Event} event 
     * @returns {boolean}
     */
    vfunc_key_press_event(event) {
        const key_symbol = event.get_key_symbol();
        if (key_symbol === Clutter.KEY_C || key_symbol === Clutter.KEY_c) {
            return super.vfunc_key_press_event(event);
        }
        return Clutter.EVENT_STOP;
    }
})
