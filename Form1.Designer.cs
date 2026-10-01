namespace MouseMoverApp;

public partial class Form1
{
    private readonly Button btnToggle = new() { Name="btnToggle",Location=new(8,8),Size=new(274,35),TabIndex=0,Text="Start (F10)",UseVisualStyleBackColor=true };
    private readonly Panel pnlIdleLamp = new() { Name="pnlIdleLamp",Location=new(8,52),Size=new(12,12),TabIndex=1,BackColor=Color.Gray,BorderStyle=BorderStyle.FixedSingle };
    private readonly Label lblIdleInfo = new() { Name="lblIdleInfo",Location=new(26,48),Size=new(256,18),TabIndex=2,AutoSize=false,TextAlign=ContentAlignment.TopLeft,Text="Not Started    Idle: 00:00  In: 03:00" };
    private readonly CheckBox chkStopCountdown = new() { Name="chkStopCountdown",Location=new(8,80),Size=new(111,25),TabIndex=3,AutoSize=true,Text="Stop countdown",UseVisualStyleBackColor=true };
    private readonly NumericUpDown numStopMinutes = new() { Name="numStopMinutes",Location=new(125,78),Size=new(42,23),TabIndex=4,Maximum=999,Value=180 };
    private readonly Label lblStopMinutes = new() { Name="lblStopMinutes",Location=new(170,82),Size=new(28,15),TabIndex=5,AutoSize=true,Text="min" };
    private readonly NumericUpDown numStopSeconds = new() { Name="numStopSeconds",Location=new(214,78),Size=new(42,23),TabIndex=6,Maximum=59 };
    private readonly Label lblStopSeconds = new() { Name="lblStopSeconds",Location=new(259,82),Size=new(24,15),TabIndex=7,AutoSize=true,Text="sec" };
    private readonly Label lblStopCountdown = new() { Name="lblStopCountdown",Location=new(8,112),Size=new(274,18),TabIndex=8,TextAlign=ContentAlignment.TopLeft,Text="Stop countdown: Off" };
    private void InitializeComponent()
    {
        SuspendLayout();
        AutoScaleDimensions = new SizeF(7,15);
        AutoScaleMode = AutoScaleMode.Font;
        ClientSize = new Size(290,138);
        Controls.AddRange([btnToggle,pnlIdleLamp,lblIdleInfo,chkStopCountdown,numStopMinutes,lblStopMinutes,numStopSeconds,lblStopSeconds,lblStopCountdown]);
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = MinimizeBox = false;
        Name = "Form1";
        Text = "AZu Sanpo";
        TopMost = true;
        btnToggle.Click += (_,_) => Toggle();
        chkStopCountdown.CheckedChanged += (_,_) => ConfigureCountdown();
        numStopMinutes.ValueChanged += (_,_) => ConfigureCountdown();
        numStopSeconds.ValueChanged += (_,_) => ConfigureCountdown();
        ResumeLayout(false);
        PerformLayout();
    }
}
