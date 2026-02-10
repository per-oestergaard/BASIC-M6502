10 REM Expression Test Program
20 REM Tests arithmetic expressions and operations
30 PRINT "EXPRESSION TESTS"
40 PRINT "================"
50 REM Simple arithmetic
60 PRINT 5+3
70 PRINT 10-4
80 PRINT 6*7
90 PRINT 20/4
100 REM Variable assignments
110 LET A = 10
120 LET B = 5
130 PRINT A
140 PRINT B
150 REM Expressions with variables
160 PRINT A+B
170 PRINT A-B
180 PRINT A*B
190 PRINT A/B
200 REM Complex expressions
210 LET X = 2
220 LET Y = 3
230 LET Z = 4
240 PRINT X+Y*Z
250 PRINT X*Y+Z
260 REM Variable reassignment
270 LET A = 100
280 PRINT A
290 LET A = A+50
300 PRINT A
310 PRINT "END OF EXPRESSION TESTS"
