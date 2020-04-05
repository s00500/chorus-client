# Chorus32 OBS Sync

This project tries to connect to a Chorus32 DroneRacing Laptimer and then sends the laptimes to OBS Studio via its websockets api. It is written in Rust

This project is still under development, but feel free to play with it, no guarantees made.


# Basic Concepts

## Basic Laptimes
For normal use one would create some scenes in OBS with Freetype Texts. These text sources should have names like "RX1LapTime" or similar and be put into the config.toml
Then add the IP address of your OBS into the config, connect the Chorus32 to your Network (Using the client mode) and add its IP to the config as well.

Then start using `cargo run`

You should be able to see your laptimes in OBS

## Masking unused Channels (very experimental)

For recieving the drones in OBS I use some Eachine Receivers. But when a drone is powered off that receiver will display noise and noise degrades the quality of the stream (cause it is though for the codec to encode a random pattern efficiently, so it takes more datarate)
Therefore I add a filter onto the camera source in OBS. The Filter should be Type Blend (Addition), 100% Opacity and Color White. Then select the bars.png from the OBSTools Folder
and name the filter eg mask. (Make sure to add the name to the Config.toml file). You can then set a threshold rssi to enable the bars overlay once the receiver powers off.

## Filemode
When the filemode option is on the laptime info is written into text files than can be fetched from OBS, so the websocket api is not needed. This used to be the simpelst way to add data into obs.


## Todo

- [x] No OBS Mode
- [x] Less errors!
- [x] OBS Error message decodinng
- [x] Initial states for race and beginning
- [ ] Finish serde config parsing (more optionals, no filemode flag in clap)
- [ ] make lap decoding inndepedent of amount
- [ ] Make parsing rssi better!
- [ ] Could add some disconection handeling of the chorus device in rssi request (if it fails 5 times churus has disconnnected and conenction should be reinitialized ?!)
