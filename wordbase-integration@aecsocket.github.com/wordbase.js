/**
 * @typedef {import("@girs/gjs")}
 * @typedef {import("@girs/gjs/dom")}
 * @typedef {import("@girs/gnome-shell/ambient")}
 * @typedef {import("@girs/gnome-shell/extensions/global")}
 * @typedef {import("@girs/soup-3.0")}
 */

import Soup from "gi://Soup";

/**
 * @typedef {Object} LookupResponse
 * @property {Object} json
 */

export default class Client {
    /** @private @type {Soup.WebsocketConnection} */
    _connection;
    /** @private @type {(function(LookupResponse): void)?} */
    _on_lookup_response;

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

                // TODO fetch config
                this._wordbase = { connection, lookup_config: { max_request_len: 16 } };

                const decoder = new TextDecoder();
                connection.connect("message", (__, message_type, message) => {
                    if (message_type != Soup.WebsocketDataType.TEXT) {
                        return;
                    }

                    const response = JSON.parse(decoder.decode(message.toArray()));
                    switch (response.type) {
                        case "Lookup":
                            /** @type {LookupResponse} */
                            // TODO parse and error handling
                            const lookup_response = response;
                            this._on_lookup_response?.(lookup_response);
                            break;
                    }
                });

                const client = new Client(connection);
                on_connect()
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
