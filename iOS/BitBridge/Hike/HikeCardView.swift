//
//  HikeCardView.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 28/1/25.
//

import SwiftUI
struct HikeCardView: View {
    var body: some View {
        ZStack {
            Circle()
                .fill(
                    LinearGradient(
                        colors: [Color("ColorIndigoMedium"), Color("ColorSalmonLight")],
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
            Image("image-1")
                .resizable()
                .scaledToFit()
        }
    }
}

struct HikeCardView_Previews: PreviewProvider {
    static var previews: some View {
        HikeCardView()
    }
}
