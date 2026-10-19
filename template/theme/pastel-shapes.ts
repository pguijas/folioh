// Original freehand contours, with matching cubic commands for a soft morph.
const contours = [
  ["M53 9C72 4 95 25 88 43C85 52 65 51 67 68C71 89 39 98 21 82C9 70 17 56 12 44C2 25 31 14 53 9Z", "M49 13C73 5 91 17 89 38C91 57 68 55 71 73C75 91 43 94 25 83C8 75 11 58 15 43C19 25 29 18 49 13Z"],
  ["M13 35C10 19 24 11 43 19C65 30 77 10 88 29C98 47 82 59 68 63C46 70 52 92 32 87C14 83 27 63 17 54C12 49 14 42 13 35Z", "M15 34C13 14 32 12 47 23C62 35 79 13 89 32C95 51 75 58 64 65C50 75 48 92 31 84C16 80 24 60 15 53C8 45 15 41 15 34Z"],
  ["M25 18C44 5 65 18 72 36C79 53 96 59 80 77C64 96 41 83 31 71C22 60 7 58 11 39C13 29 17 23 25 18Z", "M29 14C49 9 60 21 68 39C73 52 96 63 77 79C56 94 42 79 32 68C20 58 6 52 13 35C16 24 21 16 29 14Z"],
]

export function pastelShapes(seed: string) {
  const offset = Array.from(seed).reduce((hash, char) => (Math.imul(hash, 31) + char.codePointAt(0)!) >>> 0, 0) % 2
  return contours.map(paths => ({ from: paths[offset]!, to: paths[1 - offset]! }))
}

