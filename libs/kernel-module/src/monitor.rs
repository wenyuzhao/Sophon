pub trait SysMonitor {
    fn lock(&self);
    fn unlock(&self);
    fn wait(&self);
    fn notify_all(&self);
}
