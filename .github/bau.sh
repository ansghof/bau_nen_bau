#!/usr/bin/env bash
# Führt einen Bau-Befehl aus. Schlägt er fehl, kommen die Fehlerzeilen als Anmerkung an den Lauf:
# Die Protokolle selbst sieht man nur angemeldet, die Anmerkungen auf der Lauf-Seite für jeden.
log=$(mktemp)
"$@" 2>&1 | tee "$log"
status=${PIPESTATUS[0]}
if [ "$status" -ne 0 ]; then
  # Zuerst die Zeilen um jeden Fehler, sonst das Ende; überlange Zeilen (Linker-Aufrufe) gekürzt.
  { grep -E -A6 'error|undefined|cannot find|unable to|not found|FAILURE|What went wrong' "$log" || tail -n 40 "$log"; } \
    | cut -c1-300 | head -n 80 | tr -d '\r' | sed -e 's/%/%25/g' | awk '{ printf "%s%%0A", $0 }' > "$log.kurz"
  echo "::error title=$1 ist fehlgeschlagen::$(cat "$log.kurz")"
fi
exit "$status"
