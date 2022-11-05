import { ApiPromise } from '@polkadot/api';

/**
 * Match on some amount of previous polkadotjs events up to `previousBlocks` behind, executing `fn` on any event results
 * WARNING: use for tests only, as this makes use of the `events()` storage item
 * @param {Object} matchers - Used to match on the event section or method
 * @param {string} matchers.method - Method of the pallet
 * @param {string} matchers.section - Pallet name
 * @param {number} previousBlocks - How far back in block history to check
 * @callback fn - Closure to execute on the event data found for any events that match the given matcher parameters
 */
export const executeForPreviousEvent = async (
    api: ApiPromise,
    matchers: { method?: string, section?: string },
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

        events.forEach(async ({ event }: { event: any } ) => {
            // For any events, only match on combination of matchers, or single matcher
            if ('method' in matchers && 'section' in matchers) {
                if (event.method === matchers.method && event.section === matchers.section) {
                    await fn(event);
                }
            } else if ('method' in matchers && matchers.method === event.method) {
                await fn(event);
            } else if ('section' in matchers && matchers.section === event.section) {
                await fn(event);
            }
        });

        currentInHistory++;
    }
}
