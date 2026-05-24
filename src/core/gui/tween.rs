use std::time::Instant;

pub(crate) struct TweenInOutWithDelay {
    tween_time: f32,
    delay_duration: f32,
    easing: Easing,

    started: bool,
    delay_start: Option<Instant>,
}

pub(crate) enum Easing {
    OutQuad,
}

impl TweenInOutWithDelay {
    pub(crate) fn new(tween_time: f32, delay_duration: f32, easing: Easing) -> TweenInOutWithDelay {
        TweenInOutWithDelay {
            tween_time,
            delay_duration,
            easing,

            started: false,
            delay_start: None,
        }
    }

    pub(crate) fn run(&mut self, ctx: &egui::Context, id: egui::Id) -> Option<f32> {
        let anim_dir = if let Some(start) = self.delay_start {
            start.elapsed().as_secs_f32() < self.delay_duration
        } else {
            let v = self.started;
            self.started = true;
            v
        };
        let tween_val = ctx.animate_bool_with_time(id, anim_dir, self.tween_time);

        if tween_val == 1.0 && self.delay_start.is_none() {
            self.delay_start = Some(Instant::now());
        } else if tween_val == 0.0 && self.delay_start.is_some() {
            return None;
        }

        Some(match self.easing {
            Easing::OutQuad => 1.0 - (1.0 - tween_val) * (1.0 - tween_val),
        })
    }
}
