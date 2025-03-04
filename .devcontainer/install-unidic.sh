#!/bin/bash
# Downloads the Unidic dictionary and installs it into MeCab.

DOWNLOAD_PATH="$HOME/unidic.zip"

if [ ! -f "$DOWNLOAD_PATH" ]; then
    curl -L "https://clrd.ninjal.ac.jp/unidic_archive/cwj/3.1.0/unidic-cwj-3.1.0.zip" -o "$DOWNLOAD_PATH"
fi
sudo unzip -o "$DOWNLOAD_PATH" -d /usr/lib/mecab/dic
sudo mv /usr/lib/mecab/dic/unidic-cwj-3.1.1 /usr/lib/mecab/dic/unidic
sudo sed -i "s|/usr/lib/mecab/dic/ipadic|/usr/lib/mecab/dic/unidic|" /etc/mecabrc
