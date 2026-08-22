/** Small request epoch used by views whose native promises cannot be aborted. */
export class LatestRequest {
  private epoch = 0;

  begin(): number {
    this.epoch += 1;
    return this.epoch;
  }

  isCurrent(token: number): boolean {
    return token === this.epoch;
  }

  invalidate(): void {
    this.epoch += 1;
  }
}
