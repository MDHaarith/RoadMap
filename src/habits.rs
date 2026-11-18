//! Daily goal, streak and the numbers behind the Progress page. Each day the learner does something is kept in
//! activity.json as {"days": {"<day number>": {"n": steps, "right": right answers, "asked": answers}}}.
use crate::*;

const DEFAULT_GOAL: u64 = 3;

impl App {
    /// Counts `steps` toward today's goal, and `right` of `asked` answers toward the accuracy chart.
    pub(crate) fn log_activity(&self, steps: u64, right: u64, asked: u64) {
        if steps == 0 && asked == 0 {
            return;
        }
        // The usage meter's lock: callers may already hold the roadmap guard, which is not reentrant.
        let _meter = self.meter.lock().unwrap_or_else(|e| e.into_inner());
        let mut a = self.read_side("activity.json");
        if !a["days"].is_object() {
            a["days"] = json!({});
        }
        let day = &mut a["days"][self.today().to_string()];
        let n = |v: &Value, k: &str| v[k].as_u64().unwrap_or(0);
        *day = json!({"n":n(day,"n")+steps,"right":n(day,"right")+right,"asked":n(day,"asked")+asked});
        let _ = self.write_side("activity.json", &a);
    }
    pub(crate) fn goal(&self) -> u64 {
        self.config()["goal"].as_u64().filter(|g| (1..=50).contains(g)).unwrap_or(DEFAULT_GOAL)
    }
    /// Today's steps, the goal, the streak of days that met it, and the last 35 days for the home page.
    pub(crate) fn activity(&self) -> Value {
        let days = self.read_side("activity.json")["days"].clone();
        let (today, goal) = (self.today(), self.goal());
        let steps = |d: i64| days[d.to_string()]["n"].as_u64().unwrap_or(0);
        // Today still counts as open: an unmet goal today does not break the streak until the day is over.
        let first = if steps(today) >= goal { today } else { today - 1 };
        let streak = (0..).take_while(|i| steps(first - i) >= goal).count();
        let mut best = 0;
        let mut run = 0;
        let mut keys = days.as_object().into_iter().flatten().filter_map(|(k, _)| k.parse::<i64>().ok()).collect::<Vec<_>>();
        keys.sort();
        let mut last = i64::MIN;
        for d in keys {
            if steps(d) < goal {
                continue;
            }
            run = if d == last + 1 { run + 1 } else { 1 };
            best = best.max(run);
            last = d;
        }
        json!({"today":today,"goal":goal,"done":steps(today),"streak":streak,"best":best.max(streak),
            "days":(0..35).rev().map(|i| json!({"day":today-i,"n":steps(today-i)})).collect::<Vec<_>>()})
    }
    pub(crate) fn set_goal(&self, b: &Value) -> Result<Value> {
        let g = b["goal"].as_u64().filter(|g| (1..=50).contains(g)).ok_or_else(|| Error::bad("Pick a goal from 1 to 50."))?;
        let _guard = self.guard();
        self.write_config(&merge(self.config(), json!({"goal":g})))?;
        Ok(json!({}))
    }
    /// The Progress page: activity, weekly accuracy, each roadmap's progress and the weakest lessons.
    pub(crate) fn stats(&self) -> Result<Value> {
        let a = self.activity();
        let days = self.read_side("activity.json")["days"].clone();
        let today = self.today();
        // Eight weeks, oldest first; a week with no answers has no accuracy (null), so the line skips it.
        let weeks = (0..8).rev().map(|w| {
            let (mut right, mut asked) = (0, 0);
            for d in 0..7 {
                let x = &days[(today - w * 7 - d).to_string()];
                right += x["right"].as_u64().unwrap_or(0);
                asked += x["asked"].as_u64().unwrap_or(0);
            }
            json!({"week":w,"right":right,"asked":asked})
        }).collect::<Vec<_>>();
        let (mut roadmaps, mut weak) = (vec![], vec![]);
        let (mut lessons_done, mut mistakes_open) = (0, 0);
        for c in self.courses()? {
            let st = &c["state"];
            let ls = lessons(&c).collect::<Vec<_>>();
            let done = ls.iter().filter(|l| truth(&st["done"][string(&l["id"])])).count();
            lessons_done += done;
            mistakes_open += st["mistakes"].as_object().map_or(0, |m| m.values().filter(|x| !truth(&x["fixed"])).count());
            roadmaps.push(json!({"id":c["id"],"title":c["title"],"done":done,"total":ls.len(),"paused":truth(&c["paused"])}));
            for l in ls {
                let q = &st["quiz"][string(&l["id"])];
                let total = q["total"].as_f64().unwrap_or(0.);
                if total > 0. {
                    let ratio = q["score"].as_f64().unwrap_or(0.) / total;
                    if ratio < 0.7 {
                        weak.push((ratio, json!({"course":c["id"],"roadmap":c["title"],"lesson":l["id"],"title":l["title"],"score":q["score"],"total":q["total"]})));
                    }
                }
            }
        }
        weak.sort_by(|x, y| x.0.total_cmp(&y.0));
        Ok(json!({"activity":a,"weeks":weeks,"roadmaps":roadmaps,"weak":weak.into_iter().take(8).map(|w| w.1).collect::<Vec<_>>(),
            "lessons_done":lessons_done,"mistakes_open":mistakes_open}))
    }
    /// Changes when any roadmap file is added, removed or saved, so the library can reload after the MCP server writes one.
    pub(crate) fn library_stamp(&self) -> Value {
        let mut parts = fs::read_dir(&self.data).into_iter().flatten().flatten().filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let m = e.metadata().ok()?;
            let t = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_millis();
            (name.len() == 17 && name.ends_with(".json")).then(|| format!("{name}:{t}:{}", m.len()))
        }).collect::<Vec<_>>();
        parts.sort();
        json!({"stamp":parts.join("|")})
    }
}
