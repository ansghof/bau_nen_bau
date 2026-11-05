#!/usr/bin/env bash
# Führt einen Bau-Befehl aus. Schlägt er fehl, kommen die letzten Zeilen als Fehler-Anmerkung an den Lauf:
# Die Protokolle selbst sieht man nur angemeldet, die Anmerkungen auf der Lauf-Seite für jeden.
log=$(mktemp)
"$@" 2>&1 | tee "$log"
status=${PIPESTATUS[0]}
if [ "$status" -ne 0 ]; then
  message=$(grep -vE '^\s*(Compiling|Downloaded|Downloading) ' "$log" | tail -n 60 | tr -d '\r' \
    | sed -e 's/%/%25/g' | awk '{ printf "%s%%0A", $0 }')
  echo "::error title=$1 ist fehlgeschlagen::$message"
fi
exit "$status"
