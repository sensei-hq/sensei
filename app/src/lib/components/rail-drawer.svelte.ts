// The Rail as a drawer below md (#420): whether it is open, shared by the menu button that
// opens it and the Rail that draws it. From md up the Rail is always beside the content
// and this state does not move it.
class RailDrawer {
  open = $state(false);

  toggle(): void {
    this.open = !this.open;
  }

  close(): void {
    this.open = false;
  }

  /** A navigation happened: a drawer left open would cover the page just opened. */
  navigated(): void {
    this.open = false;
  }
}

export const railDrawer = new RailDrawer();
