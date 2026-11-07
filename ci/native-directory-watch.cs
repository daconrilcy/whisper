// Windows-only notification adapter for NativeBuild. No policy decisions belong here.
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;
using Microsoft.Win32.SafeHandles;

namespace Whisper.NativeWatch
{
    public sealed class Record
    {
        public string Root, Session, Volume, Name, Error;
        public uint Action;
        public ulong FileId, ParentFileId;
        public long CreationTime, Buffer, Sequence;
        public int Index;
        public string At;
    }

    public sealed class Observation
    {
        public string Type, Path, OldPath, At, Detail;
        public Record[] Records;
    }

    public sealed class Snapshot
    {
        public long Drain;
        public Record[] Records;
        public Observation[] Events;
    }

    public sealed class Collector : IDisposable
    {
        const int MaximumQueuedRecords = 262144;
        readonly object gate = new object();
        readonly List<Record> pending = new List<Record>();
        readonly List<DirectoryReader> readers = new List<DirectoryReader>();
        Record queueError;
        long sequence, drain;
        bool disposed;

        public Collector(string[] roots, bool[] recursive)
        {
            if (!OperatingSystem.IsWindowsVersionAtLeast(10, 0, 16299))
                throw new PlatformNotSupportedException("NativeBuild requires Windows 10 1709+ extended notifications.");
            if (roots == null || recursive == null || roots.Length == 0 || roots.Length != recursive.Length)
                throw new ArgumentException("Exact nonempty watcher specifications are required.");
            try
            {
                for (int i = 0; i < roots.Length; i++)
                    readers.Add(new DirectoryReader(roots[i], recursive[i], Publish));
            }
            catch { Dispose(); throw; }
        }

        void Publish(Record[] records)
        {
            lock (gate)
            {
                if (queueError != null) return; // Terminal error is retained until drained.
                if (records.Length > MaximumQueuedRecords - pending.Count)
                {
                    queueError = new Record { Error = "Notification queue capacity exceeded; coverage lost.",
                        At = DateTimeOffset.UtcNow.ToString("o"), Sequence = ++sequence };
                    return;
                }
                foreach (Record record in records) { record.Sequence = ++sequence; pending.Add(record); }
            }
        }

        public Snapshot Drain()
        {
            Record[] records;
            long id;
            lock (gate)
            {
                id = ++drain;
                records = pending.Concat(queueError == null ? Array.Empty<Record>() : new[] { queueError })
                    .Concat(readers.Select(r => r.FailureRecord()).Where(r => r != null)).ToArray();
                pending.Clear();
                // queueError is sticky: no later drain may imply recovered coverage.
            }
            return new Snapshot { Drain = id, Records = records, Events = Assemble(records) };
        }

        static Observation Failure(Record[] records, string detail) => new Observation {
            Type = "WatcherError", Path = "", OldPath = "", Detail = detail,
            At = DateTimeOffset.UtcNow.ToString("o"), Records = records };

        static string FullPath(Record record)
        {
            if (string.IsNullOrWhiteSpace(record.Root) || string.IsNullOrWhiteSpace(record.Session) ||
                string.IsNullOrWhiteSpace(record.Volume) || string.IsNullOrWhiteSpace(record.Name) ||
                System.IO.Path.IsPathRooted(record.Name) || record.Name.IndexOf('\0') >= 0 ||
                record.Name.IndexOf(':') >= 0 || record.Name.Split('\\', '/').Any(p => p == ".." || p == "." || p.Length == 0))
                throw new InvalidDataException("Invalid relative notification name or watcher identity.");
            string root = System.IO.Path.GetFullPath(record.Root).TrimEnd('\\');
            string path = System.IO.Path.GetFullPath(System.IO.Path.Combine(root, record.Name));
            if (!path.StartsWith(root + "\\", StringComparison.OrdinalIgnoreCase))
                throw new InvalidDataException("Notification escaped its watcher root.");
            return path;
        }

        // No retained halves: the caller supplies one finite snapshot of records already received.
        public static Observation[] Assemble(Record[] records)
        {
            var events = new List<Observation>();
            var renames = new Dictionary<string, List<Record>>(StringComparer.Ordinal);
            var sourceOrder = new Dictionary<string, Record>(StringComparer.Ordinal);
            long previousSequence = 0;
            foreach (Record record in records)
            {
                try
                {
                    if (!string.IsNullOrEmpty(record.Error)) throw new InvalidDataException(record.Error);
                    string path = FullPath(record);
                    if (record.Sequence <= previousSequence || record.Buffer <= 0 || record.Index < 0)
                        throw new InvalidDataException("Duplicate or unordered notification sequence.");
                    previousSequence = record.Sequence;
                    string source = record.Session + "|" + record.Volume + "|" +
                        System.IO.Path.GetFullPath(record.Root).TrimEnd('\\').ToUpperInvariant();
                    if (sourceOrder.TryGetValue(source, out Record previous) &&
                        (record.Buffer < previous.Buffer || (record.Buffer == previous.Buffer && record.Index <= previous.Index)))
                        throw new InvalidDataException("Duplicate or unordered notification buffer position.");
                    sourceOrder[source] = record;
                    if (record.Action < 1 || record.Action > 5) throw new InvalidDataException("Unknown notification action.");
                    if (record.Action == 4 || record.Action == 5)
                    {
                        if (record.FileId == 0 || record.ParentFileId == 0)
                            throw new InvalidDataException("Rename identity is missing.");
                        string key = record.Session + "|" + record.Volume + "|" + record.FileId.ToString("x16");
                        if (!renames.TryGetValue(key, out List<Record> group))
                            renames.Add(key, group = new List<Record>());
                        group.Add(record);
                    }
                    else events.Add(new Observation { Type = record.Action == 1 ? "Created" : record.Action == 2 ? "Deleted" : "Changed",
                        Path = path, OldPath = "", At = record.At, Detail = "", Records = new[] { record } });
                }
                catch (Exception ex) { events.Add(Failure(new[] { record }, ex.Message)); }
            }
            foreach (List<Record> group in renames.Values)
            {
                bool valid = group.Count % 2 == 0;
                for (int i = 0; valid && i < group.Count; i += 2)
                {
                    Record old = group[i], next = group[i + 1];
                    valid = old.Action == 4 && next.Action == 5 && old.Sequence < next.Sequence &&
                        old.CreationTime == group[0].CreationTime && next.CreationTime == group[0].CreationTime &&
                        string.Equals(old.Root, group[0].Root, StringComparison.OrdinalIgnoreCase) &&
                        string.Equals(next.Root, group[0].Root, StringComparison.OrdinalIgnoreCase);
                    if (valid && i > 0)
                    {
                        Record preceding = group[i - 1];
                        valid = preceding.Sequence < old.Sequence && preceding.ParentFileId == old.ParentFileId &&
                            string.Equals(FullPath(preceding), FullPath(old), StringComparison.OrdinalIgnoreCase);
                    }
                }
                if (!valid)
                    events.Add(Failure(group.ToArray(), "Incomplete, unordered or conflicting rename chain at drain boundary."));
                else for (int i = 0; i < group.Count; i += 2)
                {
                    Record[] pair = { group[i], group[i + 1] };
                    events.Add(new Observation { Type = "Renamed", Path = FullPath(pair[1]), OldPath = FullPath(pair[0]),
                        At = pair[0].At, Detail = "Sequential pair by watcher session, volume, file identity and creation generation.", Records = pair });
                }
            }
            // Fail before authorizing any remaining observation in an incomplete snapshot.
            return events.OrderBy(e => e.Type == "WatcherError" ? 0 : 1)
                .ThenBy(e => e.Records[0].Sequence).ToArray();
        }

        // FILE_NOTIFY_EXTENDED_INFORMATION: fixed fields end at FileName offset 84, not sizeof(struct).
        public static Record[] Parse(byte[] bytes, string root, string session, string volume, long buffer)
        {
            if (bytes == null || bytes.Length == 0) throw new InvalidDataException("Zero-byte notification: coverage lost.");
            var result = new List<Record>();
            int offset = 0;
            while (true)
            {
                int remaining = bytes.Length - offset;
                if (remaining < 84) throw new InvalidDataException("Truncated extended notification header.");
                uint next = BitConverter.ToUInt32(bytes, offset);
                uint length = BitConverter.ToUInt32(bytes, offset + 80);
                if (length == 0 || (length & 1) != 0 || length > remaining - 84)
                    throw new InvalidDataException("Invalid extended notification name length.");
                if (next != 0 && ((next & 3) != 0 || next < 84 + length || next > remaining - 84))
                    throw new InvalidDataException("Invalid next extended notification offset.");
                string name = new UnicodeEncoding(false, false, true).GetString(bytes, offset + 84, (int)length);
                result.Add(new Record { Root = root, Session = session, Volume = volume, Name = name,
                    Action = BitConverter.ToUInt32(bytes, offset + 4), CreationTime = BitConverter.ToInt64(bytes, offset + 8),
                    FileId = BitConverter.ToUInt64(bytes, offset + 64), ParentFileId = BitConverter.ToUInt64(bytes, offset + 72),
                    Buffer = buffer, Index = result.Count, At = DateTimeOffset.UtcNow.ToString("o") });
                if (next == 0) break;
                offset += (int)next;
            }
            return result.ToArray();
        }

        public void Dispose()
        {
            if (disposed) return;
            disposed = true;
            var errors = new List<Exception>();
            foreach (DirectoryReader reader in readers)
                try { reader.Dispose(); } catch (Exception ex) { errors.Add(ex); }
            if (errors.Count != 0) throw new AggregateException("Native notification shutdown failed closed.", errors);
        }
    }

    sealed class DirectoryReader : IDisposable
    {
        const uint Filter = 0x01 | 0x02 | 0x08 | 0x10; // FileName, DirectoryName, Size, LastWrite: unchanged.
        const int BufferSize = 65536;
        readonly object ioGate = new object();
        readonly Action<Record[]> publish;
        readonly string root, session = Guid.NewGuid().ToString("N");
        readonly bool recursive;
        readonly EventWaitHandle completed = new EventWaitHandle(false, EventResetMode.ManualReset);
        SafeFileHandle handle;
        Thread thread;
        IntPtr memory, overlapped;
        string volume;
        long buffer;
        bool stopping, disposed, pending;
        volatile string failure;

        public DirectoryReader(string path, bool subtree, Action<Record[]> sink)
        {
            root = Path.GetFullPath(path).TrimEnd('\\'); recursive = subtree; publish = sink;
            try
            {
                handle = CreateFileW(root, 0x0001, 7, IntPtr.Zero, 3, 0x42000000, IntPtr.Zero);
                if (handle.IsInvalid) throw new Win32Exception(Marshal.GetLastWin32Error(), "Open watcher: " + root);
                var fs = new StringBuilder(261);
                if (!GetVolumeInformationByHandleW(handle, null, 0, out uint serial, out _, out _, fs, (uint)fs.Capacity))
                    throw new Win32Exception(Marshal.GetLastWin32Error(), "Read watcher volume: " + root);
                if (!string.Equals(fs.ToString(), "NTFS", StringComparison.OrdinalIgnoreCase))
                    throw new PlatformNotSupportedException("Extended notifications require NTFS: " + root);
                volume = serial.ToString("x8");
                memory = Marshal.AllocHGlobal(BufferSize);
                overlapped = Marshal.AllocHGlobal(IntPtr.Size == 8 ? 32 : 20);
                // Thread allocation/configuration must not fail after an I/O has been armed.
                thread = new Thread(ReadLoop) { IsBackground = true, Name = "NativeBuild-directory-notifications" };
                StartArmedThread(Arm, thread.Start, CancelAndWait);
            }
            catch { Release(); throw; }
        }

        // Owns the interval between arming an I/O and transferring it to its reader thread.
        internal static void StartArmedThread(Action arm, Action start, Action cancelAndWait)
        {
            arm();
            try { start(); }
            catch { cancelAndWait(); throw; }
        }

        internal Record FailureRecord()
        {
            string message = failure;
            return message == null ? null : new Record { Root = root, Session = session, Volume = volume,
                Buffer = buffer, Error = message, At = DateTimeOffset.UtcNow.ToString("o") };
        }

        // Caller owns ioGate, or construction has not yet published the reader.
        void CancelAndWait()
        {
            if (!pending) return;
            if (!CancelIoEx(handle, overlapped) && Marshal.GetLastWin32Error() != 1168)
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Cancel pending watcher: " + root);
            if (!GetOverlappedResult(handle, overlapped, out _, true))
            {
                int error = Marshal.GetLastWin32Error();
                // Other I/O status errors are terminal completions. These two do not prove completion.
                if (error == 996 || error == 6) throw new Win32Exception(error, "Watcher completion is unproven: " + root);
            }
            pending = false;
        }

        void Arm()
        {
            if (pending) throw new InvalidOperationException("Cannot reuse an outstanding OVERLAPPED.");
            completed.Reset();
            Marshal.Copy(new byte[IntPtr.Size == 8 ? 32 : 20], 0, overlapped, IntPtr.Size == 8 ? 32 : 20);
            Marshal.WriteIntPtr(overlapped, IntPtr.Size == 8 ? 24 : 16, completed.SafeWaitHandle.DangerousGetHandle());
            if (!ReadDirectoryChangesExW(handle, memory, BufferSize, recursive, Filter, IntPtr.Zero, overlapped, IntPtr.Zero, 2))
                throw new Win32Exception(Marshal.GetLastWin32Error(), "Arm extended watcher: " + root);
            pending = true;
        }

        void ReadLoop()
        {
            try
            {
                while (true)
                {
                    completed.WaitOne();
                    byte[] bytes;
                    bool stop;
                    lock (ioGate)
                    {
                        if (!GetOverlappedResult(handle, overlapped, out uint count, false))
                        {
                            int error = Marshal.GetLastWin32Error();
                            if (stopping && error == 995) { pending = false; break; }
                            throw new Win32Exception(error, "Extended watcher completion: " + root);
                        }
                        pending = false;
                        if (count == 0 || count > BufferSize) throw new InvalidDataException("Extended notification buffer lost coverage: " + root);
                        bytes = new byte[count];
                        Marshal.Copy(memory, bytes, 0, bytes.Length);
                        stop = stopping;
                        if (!stop) Arm(); // Copy first, immediately rearm, then decode the immutable copy.
                    }
                    publish(Collector.Parse(bytes, root, session, volume, ++buffer));
                    if (stop) break;
                }
            }
            catch (Exception ex)
            {
                // Do not depend on the failing publication path to expose a terminal reader failure.
                failure = ex.Message;
            }
            finally
            {
                lock (ioGate)
                {
                    try { CancelAndWait(); Release(); }
                    catch (Exception ex) { failure = "Watcher resources retained; cancellation/completion failed: " + ex.Message; }
                }
            }
        }

        void Release()
        {
            if (pending) throw new InvalidOperationException("Watcher memory retained: I/O completion is unproven.");
            handle?.Dispose();
            if (memory != IntPtr.Zero) { Marshal.FreeHGlobal(memory); memory = IntPtr.Zero; }
            if (overlapped != IntPtr.Zero) { Marshal.FreeHGlobal(overlapped); overlapped = IntPtr.Zero; }
            completed.Dispose();
        }

        public void Dispose()
        {
            lock (ioGate)
            {
                if (disposed) return;
                disposed = true; stopping = true;
                if (!handle.IsClosed && !CancelIoEx(handle, overlapped))
                {
                    int error = Marshal.GetLastWin32Error();
                    if (error != 1168) throw new Win32Exception(error, "Cancel extended watcher: " + root);
                }
            }
            // On timeout, deliberately do not free memory still owned by pending kernel I/O.
            if (!thread.Join(5000)) throw new TimeoutException("Extended watcher did not finish cancellation: " + root);
            lock (ioGate)
                if (pending) throw new InvalidOperationException("Watcher stopped with uncompleted I/O; resources retained: " + root);
        }

        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true, ExactSpelling = true)]
        static extern SafeFileHandle CreateFileW(string path, uint access, uint share, IntPtr security, uint disposition, uint flags, IntPtr template);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true, ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool GetVolumeInformationByHandleW(SafeFileHandle file, StringBuilder name, uint nameSize,
            out uint serial, out uint maximumComponent, out uint flags, StringBuilder fileSystem, uint fileSystemSize);
        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool ReadDirectoryChangesExW(SafeFileHandle directory, IntPtr buffer, uint length,
            [MarshalAs(UnmanagedType.Bool)] bool subtree, uint filter, IntPtr returned, IntPtr overlapped, IntPtr callback, int informationClass);
        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool GetOverlappedResult(SafeFileHandle file, IntPtr overlapped, out uint transferred, [MarshalAs(UnmanagedType.Bool)] bool wait);
        [DllImport("kernel32.dll", SetLastError = true, ExactSpelling = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool CancelIoEx(SafeFileHandle file, IntPtr overlapped);
    }
}
