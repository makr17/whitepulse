# whitepulse
random pulsing sACN output, in white

another program to output to the lights mounted under the eaves at my house.
may or may not be useful to others as-is, but could point you in the right direction...

given a string of pixels, at any point in time a dark pixel has a given probability of lighting.
if it lights, it sets a random brightness between zero and MAX.
if it is already lit, the level will either rise or fall, with the probability of falling increasing as the pixel gets older.
if the level falls to zero, we reset age to zero and start the process again.
the intended effect is random pixels lighting, pulsing up, then decaying.  a pulsing random twinkle, for lack of a better description...

the constants for DECAY_FACTOR, LIGHT_THRESHOLD and MAX_BRIGHTNESS are still in flux, we'll see what looks good once the code works...
