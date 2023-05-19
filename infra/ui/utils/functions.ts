export function toHex(buffer) {
    return Array.from(buffer)
        .map((byte) => byte.toString(16).padStart(2, '0'))
        .join('')
}
