use std::fmt;
use rand::prelude::*;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::time::{Instant};


//////////////////////////// Structs & constants //////////////////////////////////////////////

const WILDCARD: u8 = 0;
const SKIP: u8 = 13;
const EMPTY_CARD: u8 = 15;
#[derive(Copy, Clone, Debug)]
struct Card {
    number: u8,
    color: u8,
}
impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.number {
            WILDCARD => write!(f, "W"),
            SKIP => write!(f, "S"),
            n @ 1..=12 => {
                let c = match self.color {
                    0 => 'R',
                    1 => 'G',
                    2 => 'B',
                    3 => 'Y',
                    _ => '?',
                };
                write!(f, "{}{}", n, c)
            }
            _ => write!(f, "?"),
        }
    }
}
#[allow(dead_code)] 
fn print_hand(hand: &[Card; 10]) {
    for (i, card) in hand.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("{}", card);
    }
    println!();
}

#[derive(Copy, Clone, Debug)]
struct HandProfile {
    counts: [u8; 14],
    colors: [u8; 5],
}
impl fmt::Display for HandProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "| W | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10| 11| 12| S || R | G | B | Y | ! |"
        )?;
        write!(f, "|")?;
        for i in 0..14 {
            write!(f, "{:>3}|", self.counts[i])?;
        }
        write!(f, "|")?;
        for i in 0..5 {
            write!(f, "{:>3}|", self.colors[i])?;
        }

        Ok(())
    }
}

const DECK_SIZE: usize = 108;
const DECK: [Card; DECK_SIZE] = build_deck();
const fn build_deck() -> [Card; DECK_SIZE] {
    let empty: Card = Card { number: EMPTY_CARD, color: 0 };
    let mut deck: [Card; DECK_SIZE] = [empty; DECK_SIZE];
    let mut i: usize = 0;

    // 8 wildcards
    let mut w: i32 = 0;
    while w < 8 {
        deck[i] = Card { number: WILDCARD, color: 4 };
        i += 1;
        w += 1;
    }

    // Numbers 1..12
    let mut num: u8 = 1;
    while num <= 12 {
        let mut col: u8 = 0;
        while col < 4 {
            let mut copies: i32 = 0;
            while copies < 2 {
                deck[i] = Card { number: num, color: col };
                i += 1;
                copies += 1;
            }
            col += 1;
        }
        num += 1;
    }

    // 4 skips
    let mut s: i32 = 0;
    while s < 4 {
        deck[i] = Card { number: SKIP, color: 4 };
        i += 1;
        s += 1;
    }

    deck
}


//////////////////////////// Hand generation //////////////////////////////////////////////////

fn random_hand() -> [Card; 10] {
    let mut rng: ThreadRng = rand::rng();

    let empty: Card = Card { number: EMPTY_CARD, color: 0 };
    let mut generated: [Card; 10] = [empty; 10];

    let mut card: usize = 0;
    let mut used: [bool; DECK_SIZE] = [false; DECK_SIZE];
    while card<10 {
        let idx: usize = rng.random_range(0..DECK_SIZE);
        if !used[idx] {
            generated[card] = DECK[idx];
            used[idx] = true;
            card+=1;
        }
    }
    generated
}

fn profile_hand(hand: &[Card; 10]) -> HandProfile {
    let mut parsed: HandProfile = HandProfile { counts: [0;14], colors: [0;5] };
    for &card in hand.iter(){
        parsed.counts[card.number as usize] += 1;
        if 0 < card.number && card.number < SKIP{
            parsed.colors[card.color as usize] += 1;
        }
        else {
            parsed.colors[4] += 1;
        }
    }
    parsed
}


//////////////////////////// Hand Evaluation //////////////////////////////////////////////////
 
fn check_2_sets(profile: &HandProfile, set1: u8, set2: u8) -> bool{
    assert!(0<set1 && 0<set2 && set1+set2<=10);
    let s1 = set1.max(set2);
    let s2 = set1.min(set2);
    let mut filled1 = false;
    let mut filled2 = false;
    for &c in profile.counts[1..=12].iter() {
        if s2 <= c && c < s1 {
            if !filled2 {
                filled2 = true;
                if filled1 { return true }
            }
        }
        else if s1 <= c {
            if !filled1 {
                filled1 = true;
                if filled2 { return true }
            }
            else if !filled2 { return true }
        }
    }

    // second pass for partial groups
    let wilds = profile.counts[0];
    let mut max1 = 0;
    let mut max2 = 0;
    for &x in profile.counts[1..=12].iter() {
        let v = x;
        if v > max1 {
            max2 = max1;
            max1 = v;
        } else if v > max2 {
            max2 = v;
        }
    }

    let a = max1.min(s1);
    let b = max2.min(s2);

    let missing = (s1-a) + (s2-b);
    if a>0 && b>0 && missing <= wilds {
        return true;
    }

    // finally check 2 sets from the same number
    let ab = max1.min(s1+s2);
    let missing = s1+s2 - ab;

    ab>1 && missing <= wilds
}

fn check_set_run(profile: &HandProfile, set: u8, run: u8) -> bool{
    assert!(0<set && 0<run && set+run<=10);
    let wilds = profile.counts[0];

    for i in 1..=12 {
        let count = profile.counts[i];
        if count < 1 { continue }

        let used_natural = count.min(set);
        let need = set.saturating_sub(count);

        if need > wilds { continue }

        let mut subprofile = profile.clone();
        subprofile.counts[i] = count - used_natural;
        subprofile.counts[0] = wilds - need;

        if check_run(&subprofile, run) {
            return true;
        }
    }
    
    false
}

fn check_run(profile: &HandProfile, run: u8) -> bool{
    assert!(0<run && run<=10);
    let wilds = profile.counts[0];

    let mut holes: i8 = -(wilds as i8);
    if wilds >= run { holes = 1-(run as i8) }
    for i in 1..=run as usize{
        let count = profile.counts[i];
        if count < 1 {
            holes+=1;
        }
    }
    if holes <= 0 {
        return true;
    }
    for i in (run as usize)+1..=12{     
        let j = i-(run as usize);
        if profile.counts[j] < 1 {
            holes-=1;
        }
        if profile.counts[i] < 1 {
            holes+=1;
        }
        if holes <= 0 {
            return true;
        }
    }
    false
}

fn check_color(profile: &HandProfile, color: u8) -> bool{
    let wilds = profile.counts[0];
    let need = if wilds >= color { 1 } else { color - wilds };
    for &c in profile.colors[0..=3].iter(){
        if c >= need{
            return true;
        }
    }
    false
}


//////////////////////////// Monte Carlo Simulation ///////////////////////////////////////////

#[allow(dead_code)] 
fn monte_carlo(phases: [CheckFunc; 10], runs: u64){
    let mut phase_counts: [u64; 10] = [0; 10]; 
    for _ in 0..runs {
        let hand = random_hand();
        let profile = profile_hand(&hand);
        for (i, f) in phases.iter().enumerate() {
            if f(&profile) {
                phase_counts[i] += 1;
            }
        }
    }

    for i in 0..10{
        let stat = (phase_counts[i] as f64 / runs as f64) * 100f64;
        println!("Phase {}: {:.4}%", i+1, stat);
    }
}

fn monte_carlo_parallel(phases: [CheckFunc; 10], runs: u64) {
    let phase_counts = (0..runs)
        .into_par_iter()
        .fold(
            || [0u64; 10], // thread-local counts
            |mut local_counts, _| {
                let hand = random_hand();
                let profile = profile_hand(&hand);

                for (i, f) in phases.iter().enumerate() {
                    if f(&profile) {
                        local_counts[i] += 1;
                    }
                }
                local_counts
            },
        )
        .reduce(
            || [0u64; 10],
            |mut a, b| {
                for i in 0..10 {
                    a[i] += b[i];
                }
                a
            },
        );

    for i in 0..10 {
        let p = phase_counts[i] as f64 / runs as f64;
        let eps = 2.576_f64 * (p * (1.0_f64 - p) / runs as f64).sqrt();

        println!(
            "Phase {}: {:.6}% (±{:.6}%)",
            i + 1,
            p * 100.0,
            eps * 100.0
        );
    }
}


//////////////////////////// Main /////////////////////////////////////////////////////////////

const MONTE_CARLO_RUNS: u64 = 1_000;
type CheckFunc = fn(&HandProfile) -> bool;

fn main() {
    let phases: [CheckFunc; 10] = [
        |p| check_2_sets(p,3,3),   
        |p| check_set_run(p,3,4),         
        |p| check_set_run(p,4,4),        
        |p| check_run(p,7),                 
        |p| check_run(p,8),                  
        |p| check_run(p,9),                  
        |p| check_2_sets(p,4,4),
        |p| check_color(p,7),
        |p| check_2_sets(p,5,2),
        |p| check_2_sets(p,5,3),
    ];
    // monte_carlo(phases, MONTE_CARLO_RUNS);
    let t0 = Instant::now();
    monte_carlo_parallel(phases, MONTE_CARLO_RUNS);
    println!("Done in: {:?}", t0.elapsed());
}


//////////////////////////// Unit Tests ///////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_2_sets(){
        let profiles: [(HandProfile, (u8, u8), bool); 38] = [
            (HandProfile { counts: [3,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [3,3,2,1, 3] }, (3,3), false),
            (HandProfile { counts: [8,0,0,0,0,0,0,0,0,0,0,0,0,2], colors: [0,0,0,0,10] }, (3,3), false),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [3,3,2,1, 1] }, (5,3), false),
            (HandProfile { counts: [0,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [3,3,3,1, 0] }, (3,5), false),
            (HandProfile { counts: [3,1,1,0,1,1,1,0,1,1,1,0,0,0], colors: [3,3,2,1, 3] }, (4,4), false),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [3,3,2,1, 3] }, (3,5), false),
            (HandProfile { counts: [1,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [3,3,3,1, 0] }, (3,5), false),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [3,3,2,1, 1] }, (2,5), false),
            (HandProfile { counts: [0,3,0,1,0,1,0,0,2,3,0,0,0,0], colors: [3,3,3,1, 0] }, (5,2), false),
            (HandProfile { counts: [0,1,2,0,1,1,1,0,1,2,1,0,0,0], colors: [3,3,0,1, 3] }, (4,2), false),
            (HandProfile { counts: [1,3,0,3,0,1,0,0,0,0,0,0,0,3], colors: [3,3,0,0, 4] }, (5,2), false),
            (HandProfile { counts: [3,1,0,1,0,1,0,1,0,0,0,0,0,0], colors: [3,3,0,1, 3] }, (7,2), false),
            (HandProfile { counts: [3,0,0,0,1,1,1,1,0,0,0,0,0,0], colors: [3,3,0,1, 3] }, (7,2), false),
            (HandProfile { counts: [3,1,1,0,1,0,1,0,0,0,0,0,0,0], colors: [3,3,0,1, 3] }, (7,2), false),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,0,0,0,0,0,0], colors: [3,3,0,1, 3] }, (5,2), false),
            (HandProfile { counts: [1,3,3,3,0,1,0,0,0,0,0,0,0,0], colors: [3,3,3,0, 1] }, (5,2), false),
            (HandProfile { counts: [0,4,4,0,0,0,1,0,0,0,1,0,0,0], colors: [3,3,3,1, 0] }, (5,3), false),
            (HandProfile { counts: [7,1,0,0,0,0,0,0,0,0,0,0,0,2], colors: [1,0,0,0, 9] }, (5,3), false),
            (HandProfile { counts: [8,0,0,0,0,0,0,0,0,0,0,0,0,2], colors: [0,0,0,0,10] }, (3,3), false),
            (HandProfile { counts: [3,1,1,1,1,1,0,0,0,0,0,0,0,2], colors: [5,1,0,0, 5] }, (7,3), false),
            (HandProfile { counts: [0,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [8,2,0,0, 0] }, (2,2), false),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [3,3,2,1, 3] }, (3,3), false),
            (HandProfile { counts: [0,3,0,0,0,0,0,0,0,0,0,0,0,7], colors: [3,0,0,0, 7] }, (3,3), false),
            (HandProfile { counts: [0,5,0,2,0,0,0,0,0,0,0,0,0,3], colors: [4,3,0,0, 3] }, (5,3), false),
            (HandProfile { counts: [0,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [3,3,3,1, 0] }, (3,3), true ),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [3,3,2,1, 1] }, (3,3), true ),
            (HandProfile { counts: [2,1,1,1,1,3,1,1,1,0,0,0,0,0], colors: [3,3,1,1, 2] }, (3,3), true ),
            (HandProfile { counts: [6,1,1,1,1,0,0,0,0,0,0,0,0,0], colors: [0,0,2,1, 6] }, (4,4), true ),
            (HandProfile { counts: [3,1,1,0,1,1,1,0,1,1,1,0,0,0], colors: [3,3,0,1, 3] }, (3,2), true ),
            (HandProfile { counts: [0,1,1,1,1,0,1,1,0,1,1,0,0,0], colors: [5,0,5,0, 0] }, (1,1), true ),
            (HandProfile { counts: [0,3,0,0,3,0,0,0,0,0,0,0,0,4], colors: [3,3,0,0, 4] }, (3,3), true ),
            (HandProfile { counts: [0,6,0,0,0,0,0,0,0,0,0,0,0,4], colors: [6,0,0,0, 4] }, (3,3), true ),
            (HandProfile { counts: [1,3,0,2,0,0,0,0,0,0,0,0,0,4], colors: [2,2,1,0, 5] }, (3,3), true ),
            (HandProfile { counts: [0,4,0,0,4,0,0,0,0,0,0,0,0,2], colors: [4,4,0,0, 2] }, (4,4), true ),
            (HandProfile { counts: [1,4,0,0,3,0,0,0,0,0,0,0,0,2], colors: [4,3,0,0, 3] }, (4,4), true ),
            (HandProfile { counts: [0,5,0,2,0,0,0,0,0,0,0,0,0,3], colors: [4,3,0,0, 3] }, (5,2), true ),
            (HandProfile { counts: [1,5,0,1,0,0,0,0,0,0,0,0,0,3], colors: [4,2,0,0, 4] }, (5,2), true ),
            (HandProfile { counts: [1,5,0,2,0,0,0,0,0,0,0,0,0,2], colors: [4,3,0,0, 3] }, (5,3), true ),
        ];
        for test in profiles.iter(){
            assert_eq!(check_2_sets(&test.0, test.1.0, test.1.1), test.2, "test {test:?}");
        }
    }

    #[test]
    fn test_check_set_run(){
        let profiles: [(HandProfile, (u8, u8), bool); 37] = [
            (HandProfile { counts: [8,0,0,0,0,0,0,0,0,0,0,0,0,2], colors: [ 0,0,0,0,10] }, (3,3), false),
            (HandProfile { counts: [0,3,3,1,1,1,0,0,0,0,0,0,0,0], colors: [ 3,3,3,1, 0] }, (3,5), false),
            (HandProfile { counts: [3,1,1,0,1,1,1,0,1,1,1,0,0,0], colors: [ 3,3,2,1, 3] }, (4,4), false),
            (HandProfile { counts: [0,3,0,1,0,1,0,0,2,3,0,0,0,0], colors: [ 3,3,3,1, 0] }, (5,2), false),
            (HandProfile { counts: [0,1,2,0,1,1,1,0,1,2,1,0,0,0], colors: [ 3,3,0,1, 3] }, (4,2), false),
            (HandProfile { counts: [1,3,0,3,0,1,0,0,0,0,0,0,0,3], colors: [ 3,3,0,0, 4] }, (5,2), false),
            (HandProfile { counts: [3,1,0,1,0,1,0,1,0,0,0,0,0,0], colors: [ 3,3,0,1, 3] }, (7,2), false),
            (HandProfile { counts: [3,0,0,0,1,1,1,1,0,0,0,0,0,0], colors: [ 3,3,0,1, 3] }, (7,2), false),
            (HandProfile { counts: [3,1,1,0,1,0,1,0,0,0,0,0,0,0], colors: [ 3,3,0,1, 3] }, (7,2), false),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,0,0,0,0,0,0], colors: [ 3,3,0,1, 3] }, (5,2), false),
            (HandProfile { counts: [1,3,3,3,0,1,0,0,0,0,0,0,0,0], colors: [ 3,3,3,0, 1] }, (5,2), false),
            (HandProfile { counts: [0,4,4,0,0,0,1,0,0,0,1,0,0,0], colors: [ 3,3,3,1, 0] }, (5,3), false),
            (HandProfile { counts: [7,1,0,0,0,0,0,0,0,0,0,0,0,2], colors: [ 1,0,0,0, 9] }, (5,3), false),
            (HandProfile { counts: [3,1,1,1,1,1,0,0,0,0,0,0,0,2], colors: [ 5,1,0,0, 5] }, (7,3), false),
            (HandProfile { counts: [0,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [ 8,2,0,0, 0] }, (2,2), false),
            (HandProfile { counts: [8,0,0,0,0,0,0,0,0,0,0,0,0,2], colors: [ 0,0,0,0,10] }, (3,4), false),
            (HandProfile { counts: [1,3,0,0,0,1,0,0,1,0,0,0,0,4], colors: [ 4,0,0,0, 6] }, (3,4), false),
            (HandProfile { counts: [0,2,2,2,2,0,0,0,0,0,0,0,0,2], colors: [ 8,0,0,0, 2] }, (4,4), false),
            (HandProfile { counts: [0,1,1,1,1,1,1,1,1,1,0,0,0,1], colors: [ 9,0,0,0, 1] }, (3,4), false),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [ 3,3,2,1, 3] }, (4,4), true ),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [ 3,3,2,1, 3] }, (3,5), true ),
            (HandProfile { counts: [1,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [ 3,3,3,1, 0] }, (3,5), true ),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [ 3,3,2,1, 1] }, (2,5), true ),
            (HandProfile { counts: [0,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [ 3,3,3,1, 0] }, (3,3), true ),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [ 3,3,2,1, 1] }, (3,3), true ),
            (HandProfile { counts: [2,1,1,1,1,3,1,1,1,0,0,0,0,0], colors: [ 3,3,1,1, 2] }, (3,3), true ),
            (HandProfile { counts: [6,1,1,1,1,0,0,0,0,0,0,0,0,0], colors: [ 0,0,2,1, 6] }, (4,4), true ),
            (HandProfile { counts: [3,1,1,0,1,1,1,0,1,1,1,0,0,0], colors: [ 3,3,0,1, 3] }, (3,2), true ),
            (HandProfile { counts: [0,1,1,1,1,0,1,1,0,1,1,0,0,0], colors: [ 5,0,5,0, 0] }, (1,1), true ),
            (HandProfile { counts: [0,3,0,0,0,1,1,1,1,0,0,0,0,3], colors: [ 7,0,0,0, 3] }, (3,4), true ),
            (HandProfile { counts: [0,0,4,0,0,0,1,1,1,1,0,0,0,2], colors: [ 8,0,0,0, 2] }, (4,4), true ),
            (HandProfile { counts: [1,3,0,0,0,1,1,0,1,0,0,0,0,3], colors: [ 5,0,0,0, 5] }, (3,4), true ),
            (HandProfile { counts: [0,3,2,2,1,1,1,0,0,0,0,0,0,1], colors: [ 9,0,0,0, 1] }, (3,4), true ),
            (HandProfile { counts: [0,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [10,0,0,0, 0] }, (3,4), true ),
            (HandProfile { counts: [0,5,1,1,0,0,0,0,0,0,0,0,0,3], colors: [ 7,0,0,0, 3] }, (5,2), true ),
            (HandProfile { counts: [1,4,1,1,0,0,0,0,0,0,0,0,0,3], colors: [ 6,0,0,0, 4] }, (5,2), true ),
            (HandProfile { counts: [0,3,0,0,0,0,0,0,0,1,1,1,1,2], colors: [ 7,0,0,0, 3] }, (3,4), true ),
        ];
        for test in profiles.iter(){
            assert_eq!(check_set_run(&test.0, test.1.0, test.1.1), test.2, "test {test:?}");
        }
    }

    #[test]
    fn test_check_run(){
        let profiles: [(HandProfile, u8, bool); 36] = [
            (HandProfile { counts: [8,0,0,0,0,0,0,0,0,0,0,0,0,2], colors: [ 0,0,0,0,10] }, 7, false),
            (HandProfile { counts: [0,3,0,1,0,1,0,0,2,3,0,0,0,0], colors: [ 3,3,3,1, 0] }, 5, false),
            (HandProfile { counts: [0,1,2,0,1,1,1,0,1,2,1,0,0,0], colors: [ 3,3,0,1, 3] }, 4, false),
            (HandProfile { counts: [1,3,0,3,0,1,0,0,0,0,0,0,0,3], colors: [ 3,3,0,0, 4] }, 5, false),
            (HandProfile { counts: [8,0,0,0,0,0,0,0,0,0,0,0,0,2], colors: [ 0,0,0,0,10] }, 3, false),
            (HandProfile { counts: [0,1,2,0,1,1,1,0,1,2,1,0,0,0], colors: [ 3,3,0,1, 3] }, 4, false),
            (HandProfile { counts: [8,0,0,0,0,0,0,0,0,0,0,0,0,2], colors: [ 0,0,0,0,10] }, 7, false),
            (HandProfile { counts: [1,1,0,1,0,1,1,0,0,0,0,0,0,4], colors: [ 5,0,0,0, 5] }, 7, false),
            (HandProfile { counts: [0,1,1,1,0,1,1,1,0,1,0,0,0,3], colors: [ 6,0,0,0, 4] }, 9, false),
            (HandProfile { counts: [0,2,0,2,0,2,0,2,0,2,0,0,0,0], colors: [10,0,0,0, 0] }, 7, false),
            (HandProfile { counts: [3,1,1,0,1,1,1,0,1,1,1,0,0,0], colors: [ 3,3,0,1, 3] }, 3, true ),
            (HandProfile { counts: [3,1,0,1,0,1,0,1,0,0,0,0,0,0], colors: [ 3,3,0,1, 3] }, 7, true ),
            (HandProfile { counts: [3,0,0,0,1,1,1,1,0,0,0,0,0,0], colors: [ 3,3,0,1, 3] }, 7, true ),
            (HandProfile { counts: [3,1,1,0,1,0,1,0,0,0,0,0,0,0], colors: [ 3,3,0,1, 3] }, 7, true ),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,0,0,0,0,0,0], colors: [ 3,3,0,1, 3] }, 5, true ),
            (HandProfile { counts: [1,3,3,3,0,1,0,0,0,0,0,0,0,0], colors: [ 3,3,3,0, 1] }, 5, true ),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [ 3,3,2,1, 1] }, 5, true ),
            (HandProfile { counts: [0,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [ 3,3,3,1, 0] }, 3, true ),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [ 3,3,2,1, 3] }, 3, true ),
            (HandProfile { counts: [1,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [ 3,3,3,1, 0] }, 3, true ),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [ 3,3,2,1, 1] }, 2, true ),
            (HandProfile { counts: [7,1,0,0,0,0,0,0,0,0,0,0,0,2], colors: [ 1,0,0,0, 9] }, 5, true ),
            (HandProfile { counts: [3,1,1,1,1,1,0,0,0,0,0,0,0,2], colors: [ 5,1,0,0, 5] }, 7, true ),
            (HandProfile { counts: [0,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [ 8,2,0,0, 0] }, 2, true ),
            (HandProfile { counts: [0,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [ 3,3,3,1, 0] }, 3, true ),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [ 3,3,2,1, 1] }, 3, true ),
            (HandProfile { counts: [2,1,1,1,1,3,1,1,1,0,0,0,0,0], colors: [ 3,3,1,1, 2] }, 3, true ),
            (HandProfile { counts: [6,1,1,1,1,0,0,0,0,0,0,0,0,0], colors: [ 0,0,2,1, 6] }, 4, true ),
            (HandProfile { counts: [3,1,1,0,1,1,1,0,1,1,1,0,0,0], colors: [ 3,3,0,1, 3] }, 3, true ),
            (HandProfile { counts: [0,1,1,1,1,0,1,1,0,1,1,0,0,0], colors: [ 5,0,5,0, 0] }, 1, true ),
            (HandProfile { counts: [0,1,1,1,1,1,1,1,0,0,0,0,0,3], colors: [ 7,0,0,0, 3] }, 7, true ),
            (HandProfile { counts: [1,1,1,1,0,1,1,1,0,0,0,0,0,2], colors: [ 6,0,0,0, 4] }, 7, true ),
            (HandProfile { counts: [0,0,0,0,0,1,1,1,1,1,1,1,1,2], colors: [ 8,0,0,0, 2] }, 8, true ),
            (HandProfile { counts: [1,1,1,1,1,1,1,1,0,1,0,0,0,1], colors: [ 7,0,0,0, 3] }, 9, true ),
            (HandProfile { counts: [0,2,2,1,1,1,1,1,0,0,0,0,0,2], colors: [ 8,0,0,0, 2] }, 7, true ),
            (HandProfile { counts: [0,0,0,0,0,0,1,1,1,1,1,1,1,3], colors: [ 7,0,0,0, 3] }, 7, true ),
        ];
        for test in profiles.iter(){
            assert_eq!(check_run(&test.0, test.1), test.2, "test {test:?}");
        }
    }

    #[test]
    fn test_check_color(){
        let profiles: [(HandProfile, u8, bool); 34] = [
            (HandProfile { counts: [0,3,0,1,0,1,0,0,2,3,0,0,0,0], colors: [3,3,3,1, 0] }, 5, false),
            (HandProfile { counts: [0,1,2,0,1,1,1,0,1,2,1,0,0,0], colors: [3,3,0,1, 3] }, 4, false),
            (HandProfile { counts: [1,3,0,3,0,1,0,0,0,0,0,0,0,3], colors: [3,3,0,0, 4] }, 5, false),
            (HandProfile { counts: [3,1,0,1,0,1,0,1,0,0,0,0,0,0], colors: [3,3,0,1, 3] }, 7, false),
            (HandProfile { counts: [3,0,0,0,1,1,1,1,0,0,0,0,0,0], colors: [3,3,0,1, 3] }, 7, false),
            (HandProfile { counts: [3,1,1,0,1,0,1,0,0,0,0,0,0,0], colors: [3,3,0,1, 3] }, 7, false),
            (HandProfile { counts: [1,3,3,3,0,1,0,0,0,0,0,0,0,0], colors: [3,3,3,0, 1] }, 5, false),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [3,3,2,1, 1] }, 5, false),
            (HandProfile { counts: [0,1,2,0,1,1,1,0,1,2,1,0,0,0], colors: [3,3,0,1, 3] }, 4, false),
            (HandProfile { counts: [0,3,0,1,0,1,0,0,2,3,0,0,0,0], colors: [3,3,3,1, 0] }, 5, false),
            (HandProfile { counts: [0;14],                        colors: [4,3,2,1, 0] }, 7, false),
            (HandProfile { counts: [0;14],                        colors: [0,0,0,0,10] }, 7, false),
            (HandProfile { counts: [0;14],                        colors: [2,2,2,2, 2] }, 7, false),
            (HandProfile { counts: [0,4,4,0,0,0,1,0,0,0,1,0,0,0], colors: [3,3,3,1, 0] }, 5, false),
            (HandProfile { counts: [7,1,0,0,0,0,0,0,0,0,0,0,0,2], colors: [1,0,0,0, 9] }, 9, false),
            (HandProfile { counts: [8,0,0,0,0,0,0,0,0,0,0,0,0,2], colors: [0,0,0,0,10] }, 7, false),
            (HandProfile { counts: [3,1,1,1,1,1,0,0,0,0,0,0,0,2], colors: [5,1,0,0, 5] }, 7, true ),
            (HandProfile { counts: [0,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [8,2,0,0, 0] }, 7, true ),
            (HandProfile { counts: [0,1,1,1,1,0,1,1,0,1,1,0,0,0], colors: [5,0,5,0, 0] }, 5, true ),
            (HandProfile { counts: [3,1,1,0,1,1,1,0,1,1,1,0,0,0], colors: [3,3,0,1, 3] }, 3, true ),
            (HandProfile { counts: [0,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [3,3,3,1, 0] }, 3, true ),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [3,3,2,1, 1] }, 3, true ),
            (HandProfile { counts: [2,1,1,1,1,3,1,1,1,0,0,0,0,0], colors: [3,3,1,1, 2] }, 3, true ),
            (HandProfile { counts: [6,1,1,1,1,0,0,0,0,0,0,0,0,0], colors: [0,0,2,1, 6] }, 4, true ),
            (HandProfile { counts: [3,1,1,0,1,1,1,0,1,1,1,0,0,0], colors: [3,3,0,1, 3] }, 3, true ),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,0,0,0,0,0,0], colors: [3,3,0,1, 3] }, 5, true ),
            (HandProfile { counts: [0,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [3,3,3,1, 0] }, 3, true ),
            (HandProfile { counts: [3,1,1,1,1,1,1,1,1,1,1,0,0,0], colors: [3,3,2,1, 3] }, 3, true ),
            (HandProfile { counts: [1,3,3,1,1,1,1,0,0,0,0,0,0,0], colors: [3,3,3,1, 0] }, 3, true ),
            (HandProfile { counts: [1,1,1,1,1,3,2,1,0,0,0,0,0,0], colors: [3,3,2,1, 1] }, 2, true ),
            (HandProfile { counts: [0;14],                        colors: [7,0,0,0, 0] }, 7, true ),
            (HandProfile { counts: [2;14],                        colors: [5,2,1,0, 2] }, 7, true ),
            (HandProfile { counts: [1,1,1,1,1,0,2,1,0,0,0,0,0,3], colors: [6,0,0,0, 4] }, 7, true ),
            (HandProfile { counts: [6,1,1,1,1,0,0,0,0,0,0,0,0,0], colors: [1,1,1,1, 6] }, 7, true ),
        ];
        for test in profiles.iter(){
            assert_eq!(check_color(&test.0, test.1), test.2, "test {test:?}");
        }
    }

}
