/** Accepts epoch seconds, epoch milliseconds, an ISO string or a Date. */
export const formatTime = (timestamp) => {
  if (timestamp === undefined || timestamp === null || timestamp === '') return '';
  const date = typeof timestamp === 'number' && timestamp < 9999999999
    ? new Date(timestamp * 1000)
    : new Date(timestamp);
  if (Number.isNaN(date.getTime())) return '';
  // The package cannot import the app's dateUtils; hosts may pass their own
  // formatted time instead if they need locale settings.
  // eslint-disable-next-line no-restricted-properties
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
};
