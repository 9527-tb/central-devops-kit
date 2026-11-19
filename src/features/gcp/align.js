/**
 * 同一个 IP 固定占同一行。某列次数不够时留空。行按地址从小到大排。
 */

function gapCell(slot) {
  return { id: "", fileKey: "", cidr: "", comment: "", status: "", slot };
}

function placeColumn(items) {
  const placed = [];
  const implicit = [];
  for (const item of items) {
    if (Number.isInteger(item.slot) && !placed[item.slot]) placed[item.slot] = item;
    else implicit.push(item);
  }
  let cursor = 0;
  for (const item of implicit) {
    while (placed[cursor]) cursor += 1;
    placed[cursor] = item;
    cursor += 1;
  }
  return placed;
}

function ipv4Number(addr) {
  return addr.split(".").reduce((total, part) => total * 256 + Number(part), 0);
}

function ipv6Parts(addr) {
  const halves = addr.split("::");
  if (halves.length > 2) return null;
  if (halves.length === 1) {
    const parts = addr.split(":").map((part) => Number.parseInt(part || "0", 16));
    return parts.length === 8 && parts.every((part) => part <= 0xffff) ? parts : null;
  }
  const left = halves[0] ? halves[0].split(":") : [];
  const right = halves[1] ? halves[1].split(":") : [];
  const missing = 8 - left.length - right.length;
  if (missing < 0) return null;
  const parts = [
    ...left.map((part) => Number.parseInt(part || "0", 16)),
    ...Array(missing).fill(0),
    ...right.map((part) => Number.parseInt(part || "0", 16)),
  ];
  return parts.length === 8 && parts.every((part) => part <= 0xffff) ? parts : null;
}

export function compareIp(left, right) {
  const a = ipKey(left);
  const b = ipKey(right);
  if (a.version !== b.version) return a.version - b.version;
  if (a.version === 4 && a.number !== b.number) return a.number - b.number;
  if (a.version === 6) {
    for (let index = 0; index < 8; index += 1) {
      if (a.parts[index] !== b.parts[index]) return a.parts[index] - b.parts[index];
    }
  }
  if (a.prefix !== b.prefix) return a.prefix - b.prefix;
  return String(a.text).localeCompare(String(b.text));
}

function ipKey(cidr) {
  const text = String(cidr || "");
  const slash = text.lastIndexOf("/");
  const addr = slash === -1 ? text : text.slice(0, slash);
  const prefix = slash === -1 ? 65535 : Number(text.slice(slash + 1));
  if (/^\d{1,3}(\.\d{1,3}){3}$/.test(addr)) {
    return { version: 4, number: ipv4Number(addr), prefix, text };
  }
  const parts = ipv6Parts(addr);
  if (parts) return { version: 6, parts, prefix, text };
  return { version: 255, number: 0, parts: [], prefix, text };
}

export function alignIpLists(lists) {
  const width = lists.length;
  const groups = new Map();
  lists.forEach((list, column) => {
    for (const item of list) {
      if (!item?.cidr) continue;
      if (!groups.has(item.cidr)) {
        groups.set(item.cidr, Array.from({ length: width }, () => []));
      }
      groups.get(item.cidr)[column].push(item);
    }
  });
  const rows = [];
  for (const cidr of [...groups.keys()].sort(compareIp)) {
    const placed = groups.get(cidr).map(placeColumn);
    const height = Math.max(...placed.map((items) => items.length));
    for (let index = 0; index < height; index += 1) {
      rows.push({ cells: placed.map((items) => items[index] || gapCell(index)) });
    }
  }
  return rows;
}
