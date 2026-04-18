# Monte-Carlo-for-Phase-10

This is a Monte Carlo method experiment that calculates what percentage of the sampled hands satisfy each phase in the phase 10 board game.

It consists of a rust script that contains the necessary methods to sample, profile and check multiple random 10-card hands. It is mostly an exercise in rust as I am still a novice in the language. The script processes 0.5 trillion samples (1% of total possible) in about 1.5 hours on a system with an Intel(R) Core(TM) i9-14900K and yields the following results:

|Phase| Hands       | Est. error |
| --: | ----------: | ---------: |      
|1    |  18.275229% | ±0.000141% |
|2    |  29.809240% | ±0.000167% |
|3    |  5.341861%  | ±0.000082% |
|4    |  15.568861% | ±0.000132% |
|5    |  6.488136%  | ±0.000090% |
|6    |  1.992557%  | ±0.000051% |
|7    |  0.332137%  | ±0.000021% |
|8    |  2.767219%  | ±0.000060% |
|9    |  3.112314%  | ±0.000063% |
|10   |  0.352300%  | ±0.000022% |

This is equivalent to the probability of drawing a hand that satisfies each phase right of the bat. But also statistically shows how many of all the possible hands are able to satisfy each phase and thus how difficult each phase is. This means that starting from a random hand, it is more probable that you can reach a hand that satisfies Phase 2 than Phase 1, since there are more hands that can do that.

To use the script you can compile it and run it with cargo, set the MONTE_CARLO_RUNS constant near the main method to the number of samples you desire. You can also use the single thread monte_carlo function instead of monte_carlo_parallel.
