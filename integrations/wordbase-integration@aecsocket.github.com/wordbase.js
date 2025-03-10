/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 * @typedef {import("@girs/soup-3.0")}
 */

import Soup from "gi://Soup";
import GLib from "gi://GLib";

/**
 * @typedef {Object} LookupConfig
 * @property {number} max_request_len
 *
 * @typedef {Object} HookSentence
 * @property {string} process_path
 * @property {string} sentence
 *
 * @typedef {Object} ShowPopupRequest
 * @property {number} pid
 * @property {number[]} origin
 * @property {PopupAnchor} anchor
 * @property {string} text
 *
 * @typedef {"TopLeft" | "TopRight" | "TopCenter" | "MiddleLeft" | "MiddleRight" | "BottomLeft" | "BottomCenter" | "BottomRight"} PopupAnchor
 *
 * @typedef {Object} ShowPopupResponse
 * @property {number} chars_scanned
 */

export class Client {
    /** @private @type {Soup.WebsocketConnection} */
    _connection;
    /** @private @type {LookupConfig} */
    _lookup_config;
    /** @type {(function(HookSentence): void)?} */
    on_hook_sentence;
    /** @type {(function(ShowPopupResponse): void)?} */
    _on_popup_response;

    /**
     * @param {Soup.WebsocketConnection} connection
     * @param {LookupConfig} lookup_config
     */
    constructor(connection, lookup_config) {
        this._connection = connection;
        this._lookup_config = lookup_config;
        this.on_hook_sentence = null;
        this._on_popup_response = null;
    }

    /**
     * @param {Soup.Session} soup
     * @param {string} url
     * @param {function(any): void} on_error
     * @param {function(Client): void} on_connect
     */
    static connect(soup, url, on_error, on_connect) {
        soup.websocket_connect_async(
            new Soup.Message({
                method: "GET",
                uri: GLib.Uri.parse(url, GLib.UriFlags.NONE),
            }),
            "127.0.0.1",
            [],
            0,
            null,
            (__, res) => {
                let connection;
                try {
                    connection = soup.websocket_connect_finish(res);
                } catch (err) {
                    on_error(err);
                    return;
                }

                const decoder = new TextDecoder();
                handshake(connection, decoder, on_error, (lookup_config) => {
                    const client = new Client(connection, lookup_config);

                    connection.connect("message", (__, message_type, data) => {
                        if (message_type != Soup.WebsocketDataType.TEXT) {
                            return;
                        }

                        const message = JSON.parse(
                            decoder.decode(data.toArray()),
                        );
                        switch (message.type) {
                            case "HookSentence":
                                const hook_sentence = message;
                                client.on_hook_sentence?.(hook_sentence);
                                break;
                            case "ShowPopup":
                                const response = message;
                                client._on_popup_response?.(response);
                                break;
                        }
                    });

                    on_connect(client);
                });
            },
        );
    }

    get connection() {
        return this._connection;
    }

    get lookup_config() {
        return this._lookup_config;
    }

    /**
     * @param {ShowPopupRequest} request
     * @param {function(ShowPopupResponse): void} on_response
     */
    show_popup(request, on_response) {
        this._on_popup_response = on_response;
        this._connection.send_text(
            JSON.stringify({
                type: "ShowPopup",
                ...request,
            }),
        );
    }
}

/**
 * @param {Soup.WebsocketConnection} connection
 * @param {TextDecoder} decoder
 * @param {function(any): void} on_error
 * @param {function(LookupConfig): void} on_handshake
 */
function handshake(connection, decoder, on_error, on_handshake) {
    let signal_id;
    signal_id = connection.connect("message", (__, message_type, data) => {
        connection.disconnect(signal_id);

        if (message_type != Soup.WebsocketDataType.TEXT) {
            on_error("received non-text message");
            return;
        }

        const message = JSON.parse(decoder.decode(data.toArray()));
        if (message.type === "SyncLookupConfig") {
            log(
                `Received lookup config ${JSON.stringify(message.lookup_config)}`,
            );

            on_handshake(message.lookup_config);
        }
    });
}
