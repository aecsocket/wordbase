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
 * @typedef {Object} HookSentence
 * @property {string} process_path
 * @property {string} sentence
 * 
 * @typedef {Object} LookupResponse
 * @property {Object} json
 */

export class Client {
    /** @private @type {Soup.WebsocketConnection} */
    _connection;
    /** @type {(function(HookSentence): void)?} */
    on_hook_sentence;
    /** @private @type {(function(LookupResponse): void)?} */
    _on_lookup_response;

    /**
     * @param {Soup.WebsocketConnection} connection 
     */
    constructor(connection) {
        this._connection = connection;
        this.on_hook_sentence = null;
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
                const client = new Client(connection);

                const decoder = new TextDecoder();
                connection.connect("message", (__, message_type, data) => {
                    if (message_type != Soup.WebsocketDataType.TEXT) {
                        return;
                    }

                    const message = JSON.parse(decoder.decode(data.toArray()));
                    switch (message.type) {
                        case "HookSentence":
                            // TODO parse and error handling
                            /** @type {HookSentence} */
                            const hook_sentence = message;
                            client.on_hook_sentence?.(hook_sentence);
                            break;
                        case "Lookup":
                            // TODO parse and error handling
                            /** @type {LookupResponse} */
                            const lookup_response = message;
                            client._on_lookup_response?.(lookup_response);
                            break;
                    }
                });

                on_connect(client);
            },
        );
    }

    get connection() {
        return this._connection;
    }

    /**
     * @param {string} text
     * @param {function(LookupResponse): void} on_response
     */
    lookup(text, on_response) {
        this._on_lookup_response = on_response;
        this._connection.send_text(JSON.stringify({
            type: "Lookup",
            text,
        }));
    }
}
