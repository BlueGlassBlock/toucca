using System;
using System.Collections;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Net.Http;
using System.Net.WebSockets;
using System.IO.Ports;
using System.Runtime.CompilerServices;
using System.Text;
using System.Threading.Tasks;
using System.Diagnostics;

namespace toucca
{
    public class SerialManager
    {
        const byte CMD_GET_SYNC_BOARD_VER = 0xa0;
        const byte CMD_NEXT_READ = 0x72;
        const byte CMD_GET_UNIT_BOARD_VER = 0xa8;
        const byte CMD_MYSTERY1 = 0xa2;
        const byte CMD_MYSTERY2 = 0x94;
        const byte CMD_START_AUTO_SCAN = 0xc9;
        // const byte CMD_BEGIN_WRITE = 0x77;
        // const byte CMD_NEXT_WRITE = 0x20;

        private Thread _sendThread;
        public AutoResetEvent TouchEvent = new(false);

        private static SerialPort ComL = new("COM5", 115200);
        private static SerialPort ComR = new("COM6", 115200);

        bool init = false;
        const string SYNC_BOARD_VER = "190523";
        const string UNIT_BOARD_VER = "190514";
        const string read1 = "    0    0    1    2    3    4    5   15   15   15   15   15   15   11   11   11";
        const string read2 = "   11   11   11  128  103  103  115  138  127  103  105  111  126  113   95  100";
        const string read3 = "  101  115   98   86   76   67   68   48  117    0   82  154    0    6   35    4";
        private readonly Dictionary<byte, List<byte>> readMap = new() {
            { 0x31, ByteHelper.ConvertStringToByteArray(read1) },
            { 0x32, ByteHelper.ConvertStringToByteArray(read2) },
            { 0x33, ByteHelper.ConvertStringToByteArray(read3) }
        };
        // private readonly byte[] SettingData_160 = new byte[8] { 160, 49, 57, 48, 53, 50, 51, 44 };
        private readonly byte[] SettingData_162 = [162, 63, 29];
        private readonly byte[] SettingData_148 = [148, 0, 20];
        private readonly byte[] SettingData_201 = [201, 0, 73];
        private readonly byte[] TouchPackL = new byte[36];
        private readonly byte[] TouchPackR = new byte[36];

        public void Start()
        {
            try
            {
                ComL.Open();
                ComR.Open();
            }
            catch (Exception ex)
            {
                Logger.Fatal("Failed to open serial ports", ex);
                throw;
            }
            Task.Run(PeriodicReadPortLoop);
            _sendThread = new Thread(SendLoop);
        }

        private void SendLoop()
        {
            while (true)
            {
                TouchEvent.WaitOne(1000);
                SendTouchState();
            }
        }

        public void SetTouch(int area, bool state) 
        {
            area++; // area: 1 - 240
            if (area < 121)
            {
                area += (area - 1) / 5 * 3 + 7;
                TouchPackR.SetBit(area, state);
            }
            else
            {
                area -= 120;
                area += (area - 1) / 5 * 3 + 7;
                TouchPackL.SetBit(area, state);
            }
        }

        private void SendTouchState()
        {
            if (!init)
            {
                return;
            }
            ComL.Write(ToTouchPack(TouchPackL), 0, 36);
            ComR.Write(ToTouchPack(TouchPackR), 0, 36);
        }

        private static byte[] ToTouchPack(byte[] Pack)
        {
            Pack[0] = 129;
            Pack[34]++;
            Pack[35] = 128;
            Pack[35] = ByteHelper.CalCheckSum(Pack, 36);
            if (Pack[34] > 127)
                Pack[34] = 0;
            return Pack;
        }
        private async Task PeriodicReadPortLoop()
        {
            while (true)
            {
                if (ComL.IsOpen)
                    ReadAndResp(ComL, 0);
                if (ComR.IsOpen)
                    ReadAndResp(ComR, 1);
                await Task.Delay(16);
            }
        }
        private void ReadAndResp(SerialPort Serial, int side)
        {
            if (Serial.BytesToRead <= 0)
                return;
            byte inByte = Convert.ToByte(Serial.ReadByte());
            string data = Serial.ReadExisting();

            List<byte> respBytes = new();
            switch (inByte)
            {
                case CMD_GET_SYNC_BOARD_VER:
                    init = false;
                    respBytes.Add(inByte);
                    respBytes.AddRange(ByteHelper.ConvertStringToByteArray(SYNC_BOARD_VER));
                    respBytes.Add(44);

                    break;
                case CMD_NEXT_READ:
                    init = false;
                    if (readMap.TryGetValue(Convert.ToByte(data[2]), out respBytes))
                    {
                        respBytes.Add(ByteHelper.CalCheckSum(respBytes.ToArray(), respBytes.Count));
                    }
                    else return;
                    break;
                case CMD_GET_UNIT_BOARD_VER:
                    init = false;
                    byte sideByte = side == 0 ? Convert.ToByte('R') : Convert.ToByte('L');
                    byte unitCheckSum = side == 0 ? (byte)118 : (byte)104;
                    respBytes.Add(inByte);
                    respBytes.AddRange(ByteHelper.ConvertStringToByteArray(SYNC_BOARD_VER));
                    respBytes.Add(sideByte);
                    for (int i = 0; i < 6; ++i)
                    {
                        respBytes.AddRange(ByteHelper.ConvertStringToByteArray(UNIT_BOARD_VER));
                    }
                    respBytes.Add(unitCheckSum);
                    break;
                case CMD_MYSTERY1:
                    init = false;
                    respBytes.AddRange(SettingData_162);
                    break;
                case CMD_MYSTERY2:
                    init = false;
                    respBytes.AddRange(SettingData_148);
                    break;
                case CMD_START_AUTO_SCAN:
                    respBytes.AddRange(SettingData_201);
                    init = true;
                    if (!_sendThread.IsAlive)
                    {
                        _sendThread.Start();
                    }
                    break;
                case 154:
                    init = false;
                    Logger.Warn("BAD");
                    break;

            }
            Serial.Write(respBytes.ToArray(), 0, respBytes.Count);
        }
    }

    internal static class ByteHelper
    {
        public static void SetBit(this byte[] self, int index, bool value)
        {
            var bitArray = new BitArray(self);
            bitArray.Set(index, value);
            bitArray.CopyTo(self, 0);
        }
        public static byte CalCheckSum(byte[] _PacketData, int PacketLength)
        {
            Byte _CheckSumByte = 0x00;
            for (int i = 0; i < PacketLength; i++)
                _CheckSumByte ^= _PacketData[i];
            return _CheckSumByte;
        }
        public static List<byte> ConvertStringToByteArray(string data)
        {
            List<byte> tempList = new List<byte>(100);
            for (int i = 0; i < data.Length; i++)
                tempList.Add(Convert.ToByte(data[i]));
            return tempList;
        }
    }
}
