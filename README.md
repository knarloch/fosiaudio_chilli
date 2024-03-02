# fosiaudio_chilli

A http server with simple controls:
 * play my favourite internet radio `ChilliZet`
 * pause it
 * volume control

Intended to be part of https://github.com/knarloch/fosiaudio .

Has some hardcodes and assumptions about the OS it's running on:
* `cvlc` is available, and if started as root, `cvlc` also is usable for root
* volume control is executed on *SoftMaster* alsa audio device
* 