import { ApiPromise } from '@polkadot/api';

/**
 * Match on some previous amount up to `previousBlocks` behind, executing `fn` on any event results
 * WARNING: use for tests only, as this makes use of the `events()` storage item
 * @param {Object} matchers - Used to match on the event section or method
 * @param {string} matchers.method
 * @param {string} matchers.section
 * @param {number} previousBlocks - How far back in block history to check
 * @callback fn - Closure to execute on the event data found for any events that match the given matcher parameters
 */
export const executeForPreviousEvent = async (
    api: ApiPromise,
    matchers: { method: string, section: string },
    previousBlocks: number,
    fn: (retrievedEventData: any) => any
) => {
    const currentHash = await api.rpc.chain.getBlockHash();
    let parentHash: any = currentHash;

    let currentInHistory = 0;
    while (currentInHistory !== previousBlocks) {
        let events;
        if (parentHash === null) {
            events = await api.query.system.events();
            // Set initial parentHash
            parentHash = await api.query.system.parentHash();

        } else {
            events = await api.query.system.events.at(parentHash);
            // new parentHash for next iteration
            parentHash = await api.query.system.parentHash.at(parentHash);
        }

        events.forEach(async (event: any) => {
            if (event.method === matchers.method || event.section === matchers.section) {
                await fn(event);
            } else {
                await fn({});
            }
        })
        currentInHistory++;
    }
}