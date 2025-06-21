//
//  Text.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SwiftUI

struct Heading1: ViewModifier {
    func body(content: Content) -> some View {
        content
            .lineSpacing(12)
            .font(FontTheme.H2Heading)
    }
}

struct Body1: ViewModifier {
    func body(content: Content) -> some View {
        content.lineSpacing(8)
            .font(FontTheme.Body1)
    }
}

struct Body2: ViewModifier {
    func body(content: Content) -> some View {
        content.lineSpacing(8)
            .font(FontTheme.Body2)
    }
}

struct Label1: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(FontTheme.Body2)
            .fontWeight(.bold)
    }
}

struct Label2: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(FontTheme.Body2)
    }
}

struct GradientHeading: ViewModifier {
    func body(content: Content) -> some View {
        content
            .modifier(Heading1())
            .foregroundStyle(Theme.gradientHeading)
            .multilineTextAlignment(.leading)
    }
}

struct GradientHeadingView: View {
    var body: some View {
        Text("Demo text ah dj dajw dj tn dawj dawjnt jn ")
            .frame(maxWidth: .infinity)
            .font(FontTheme.H1Heading)
            .modifier(GradientHeading())
    }
}

struct GradientHeadingView_Preview: PreviewProvider {
    static var previews: some View {
        VStack {
            Spacer()
            GradientHeadingView()
                .padding()
            Spacer()
        }
        .frame(width: .infinity, height: .infinity)
        .background(Theme.DarkViolet.color)
    }
}
