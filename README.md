# ESP32 I2S sound generator example

This demonstrates how to use the `esp-idf-hal` I2S driver under development over at
[this `esp-idf-hal` issue](https://github.com/esp-rs/esp-idf-hal/issues/205#issuecomment-1483075991).

To use it, you _will_ need the ESP-IDF 5.0 SDK checked out and your `$IDF_PATH` environment
variable pointing to it, e.g.:

```
cd ~/esp
git clone -b v5.0.1 https://github.com/espressif/esp-idf.git 
./install.sh
export IDF_PATH=$HOME/esp/esp-idf
```

You will also need to source your `export-esp.sh` script to configure the Rust ESP environment
as usual.

## Status

Currently, I am testing this on a [SparkFun Thing Plus (ESP32-S2 WROOM)](https://www.sparkfun.com/products/17743)
connected to a [MAX98357A audio breakout board](https://www.sparkfun.com/products/14809) writed to a
[TRRS audio jack](https://www.sparkfun.com/products/11570). Data signals are being written properly to the MAX98357A,
but audio is not being generated as I suspect I may have blown up my breakout board.
