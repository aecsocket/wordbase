/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 */

import St from "gi://St";
import Clutter from "gi://Clutter";
import Meta from "gi://Meta";
import GObject from "gi://GObject";
import Atk from "gi://Atk";

/**
 * @typedef {Object} Drag
 * @property {Clutter.Grab} grab
 * @property {Clutter.InputDevice} grabbed_device
 * @property {Clutter.EventSequence} grabbed_sequence
 * @property {[number, number]} pointer_origin
 * @property {Mode} mode
 *
 * @typedef {Move | Resize} Mode
 *
 * @typedef {Object} Move
 * @property {"move"} type
 * @property {[number, number]} original_position
 * @property {[number, number]} original_size
 *
 * @typedef {Object} Resize
 * @property {"resize"} type
 * @property {Direction} direction
 * @property {[number, number]} original_nw
 * @property {[number, number]} original_se
 *
 * @typedef {"se" | "sw" | "ne" | "nw"} Direction
 */

export const DragArea = GObject.registerClass(
    {
        // <https://gnome.pages.gitlab.gnome.org/libadwaita/doc/1-latest/class.Window.html>
        // AdwWindow defaults to the minimum size of 360×200 px.
        // We default to half of that.
        Properties: {
            "width-request": GObject.ParamSpec.int(
                "width-request",
                null,
                null,
                GObject.ParamFlags.READWRITE,
                0,
                0x7fffffff,
                180,
            ),
            "height-request": GObject.ParamSpec.int(
                "height-request",
                null,
                null,
                GObject.ParamFlags.READWRITE,
                0,
                0x7fffffff,
                100,
            ),
        },
        Signals: {
            "drag-begin": {},
            "drag-end": {},
        },
    },
    class Area extends St.Bin {
        /** @type {Drag?} */
        _drag;

        _init(params) {
            super._init({
                style_class: "area",
                can_focus: true,
                reactive: true,
                track_hover: true,
                hover: false,
                accessible_role: Atk.Role.WINDOW,
                ...params,
            });
            this._drag = null;
            this.set_size(
                Math.max(this.min_width, this.width_request),
                Math.max(this.min_height, this.height_request),
            );
        }

        /**
         * @param {Clutter.Event} event
         * @returns {boolean}
         */
        vfunc_button_press_event(event) {
            this._drag_begin(event);
            return Clutter.EVENT_STOP;
        }

        /**
         * @param {Clutter.Event} event
         * @returns {boolean}
         */
        vfunc_motion_event(event) {
            return this._drag_motion(event);
        }

        /**
         * @returns {boolean}
         */
        vfunc_button_release_event() {
            return this._drag_end();
        }

        /**
         * @param {Clutter.Event} event
         * @returns {boolean}
         */
        vfunc_touch_event(event) {
            if (!this._drag) {
                if (event.type() === Clutter.EventType.TOUCH_BEGIN) {
                    this._drag_begin(event);
                }
                return Clutter.EVENT_STOP;
            }

            if (
                this._drag.grabbed_sequence.get_slot() ===
                event.get_event_sequence().get_slot()
            ) {
                if (event.type() === Clutter.EventType.TOUCH_UPDATE) {
                    this._drag(event);
                    return Clutter.EVENT_STOP;
                } else if (event.type() === Clutter.EventType.TOUCH_END) {
                    this._drag_end(event);
                    return Clutter.EVENT_STOP;
                }
            }

            return Clutter.EVENT_PROPAGATE;
        }

        /**
         * @param {Clutter.Event} event
         * @returns {boolean}
         */
        _drag_begin(event) {
            if (this._drag) {
                return Clutter.EVENT_PROPAGATE;
            }

            const pointer = event.get_coords();
            const position = this.get_position();
            const size = this.get_size();

            const non_primary_button =
                Clutter.ModifierType.BUTTON2_MASK |
                Clutter.ModifierType.BUTTON3_MASK;
            /** @type {Mode} */
            let mode;
            if ((event.get_state() & non_primary_button) !== 0) {
                const abs_position = this.get_transformed_position();
                const midpoint = [
                    abs_position[0] + size[0] / 2,
                    abs_position[1] + size[1] / 2,
                ];

                /** @type {Direction} */
                let direction;
                if (pointer[1] >= midpoint[1]) {
                    if (pointer[0] >= midpoint[0]) {
                        direction = "se";
                        global.display.set_cursor(Meta.Cursor.SE_RESIZE);
                    } else {
                        direction = "sw";
                        global.display.set_cursor(Meta.Cursor.SW_RESIZE);
                    }
                } else {
                    if (pointer[0] >= midpoint[0]) {
                        direction = "ne";
                        global.display.set_cursor(Meta.Cursor.NE_RESIZE);
                    } else {
                        direction = "nw";
                        global.display.set_cursor(Meta.Cursor.NW_RESIZE);
                    }
                }

                mode = {
                    type: "resize",
                    direction,
                    original_nw: position,
                    original_se: [position[0] + size[0], position[1] + size[1]],
                };
            } else {
                mode = {
                    type: "move",
                    original_position: position,
                    original_size: size,
                };
            }

            this._drag = {
                grab: global.stage.grab(this),
                grabbed_device: event.get_device(),
                grabbed_sequence: event.get_event_sequence(),
                pointer_origin: pointer,
                mode,
            };
            this.emit("drag-begin");
            return Clutter.EVENT_STOP;
        }

        /**
         * @returns {boolean}
         */
        _drag_end() {
            if (!this._drag) {
                return Clutter.EVENT_PROPAGATE;
            }

            this._drag.grab.dismiss();
            this.emit("drag-end");
            this._drag = null;
            global.display.set_cursor(Meta.Cursor.DEFAULT);
            return Clutter.EVENT_STOP;
        }

        /**
         * @param {Clutter.Event} event
         * @returns {boolean}
         */
        _drag_motion(event) {
            if (!this._drag) {
                return Clutter.EVENT_PROPAGATE;
            }

            const pointer = event.get_coords();
            const delta = [
                pointer[0] - this._drag.pointer_origin[0],
                pointer[1] - this._drag.pointer_origin[1],
            ];

            if (this._drag.mode.type === "move") {
                this.set_position(
                    this._drag.mode.original_position[0] + delta[0],
                    this._drag.mode.original_position[1] + delta[1],
                );
            } else if (this._drag.mode.type === "resize") {
                let multiplier;
                switch (this._drag.mode.direction) {
                    case "se":
                        multiplier = [
                            [0, 0],
                            [1, 1],
                        ];
                        break;
                    case "sw":
                        multiplier = [
                            [1, 0],
                            [0, 1],
                        ];
                        break;
                    case "ne":
                        multiplier = [
                            [0, 1],
                            [1, 0],
                        ];
                        break;
                    case "nw":
                        multiplier = [
                            [1, 1],
                            [0, 0],
                        ];
                        break;
                }

                let new_nw = [
                    this._drag.mode.original_nw[0] +
                        delta[0] * multiplier[0][0],
                    this._drag.mode.original_nw[1] +
                        delta[1] * multiplier[0][1],
                ];
                let new_se = [
                    this._drag.mode.original_se[0] +
                        delta[0] * multiplier[1][0],
                    this._drag.mode.original_se[1] +
                        delta[1] * multiplier[1][1],
                ];

                const size_delta = [
                    Math.max(0, this.width_request - (new_se[0] - new_nw[0])),
                    Math.max(0, this.height_request - (new_se[1] - new_nw[1])),
                ];

                new_nw[0] -= size_delta[0] * multiplier[0][0];
                new_se[0] += size_delta[0] * multiplier[1][0];

                new_nw[1] -= size_delta[1] * multiplier[0][1];
                new_se[1] += size_delta[1] * multiplier[1][1];

                this.set_position(new_nw[0], new_nw[1]);
                this.set_size(new_se[0] - new_nw[0], new_se[1] - new_nw[1]);
            }

            return Clutter.EVENT_STOP;
        }
    },
);
