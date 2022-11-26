# Notes

## Windows

On Windows C++'s std::chrono uses a fixed list of leap seconds until 2018:
https://github.com/microsoft/STL/blob/e28f9561233a58d48d893094ed3a6bc0c5ee6ad9/stl/inc/chrono#L2120
and then dynamically looks up further (atm none) leap seconds from the registry:
https://github.com/microsoft/STL/blob/e28f9561233a58d48d893094ed3a6bc0c5ee6ad9/stl/src/tzdb.cpp#L569

The Windows team is looking into adding a proper API (or something?):
https://github.com/microsoft/STL/discussions/1624

## Linux

There directly is support for querying the TAI clock. For conversion there is no
API, but there's various files that contain the leap seconds:
https://github.com/chmike/posix_tai_time_converter
